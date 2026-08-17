# F42C3-BLAST — настоящий радиус реэкспорта, измеренный компилятором

**Чем мерил:** компилятор — три изолированные пробы (A: словари, B:
запись и проекции, C: манифест каталога), каждая заменой определений
`crates/vibe-index/src/types/**` на `pub use` из
`crates/vibe-wire/src/generated/` и прогоном
`cargo check --workspace --all-targets`. Правки — одноразовая проба,
после каждой файлы возвращены к исходному тексту (байтовый diff с
сохранённой копией — не git; плюс контрольный зелёный прогон).
Чтение не меряет здесь (F42C-REEXPORT недосчиталась — см. §Deviations),
компилятор перечисляет все сайты.

**Дата:** 2026-08-17. Вход: зелёный (`cargo check -p vibe-index
--all-targets`, `EXIT=0`).

---

## ВЕРДИКТ

**Все три пробы красные, и радиус каждой замкнут в самом `vibe-index`.**
Ни один крейт workspace'а не зависит от `vibe-index` (проверено: `vibe-index`
упоминает только собственный `Cargo.toml`), поэтому «радиус» — это не граф
крейтов, а число файлов/сайтов внутри одного крейта:

| проба | что заменено | EXIT | строк ошибок | уникальных сайтов | файлов | крейтов |
|---|---|---|---|---|---|---|
| A — словари | `PackageKind`, `NamingConvention` | **101** | **43** | 42 | 17 | 1 (`vibe-index`) |
| B — запись | `VersionEntry` + 6 relations + 6 content + 3 агрегата | **101** | **126** | 125 | 26 | 1 (`vibe-index`) |
| C — манифест | `Repomd`, `RepomdFileEntry` | **101** | **40** | 29 | 5 | 1 (`vibe-index`) |

Три главные причины (по сайтам всех проб, детали ниже): **орфанное правило**
(impl'ы нельзя держать на чужом типе — прямые E0116/E0117 и все их следствия
в виде пропавших методов/констант: 16+54+26 = 96 ошибок), **модульная
несовместимость** (тип из одного сгенерированного модуля не подходит на место
другого: 39), **смена типа поля** (семь подструктур записи стали `Option`:
23). Потеря `Copy` даёт 22 сайта (проба A), потеря `Default` — 15 (1+14),
потеря `ValueEnum` — 3, открытый словарь (`Unknown`-вариант) — 1.
Зелёных прогонов нет — замена «как есть» не собирается ни в одной из
трёх точек.

**Калибровка приёма (§7 пакета).** Счётная команда:

```sh
grep -E "^error(\[E[0-9]+\])?:" ФАЙЛ | grep -vE "aborting due to|could not compile" | wc -l
```

на зелёном выводе разогрева отвечает **0**; на красных выводах — **43 /
126 / 40**. Приём различает зелёное и красное.

---

## ПРОБА A — словари (`kinds.rs` → `entry::PackageKind`, `repomd::NamingConvention`)

### (а) Сводка

`EXIT=101` (взят как `EXIT=$?` сразу после команды, файл `/tmp/blast-a.exit`).
Строк `error[…]` **43**, строк `error:` без кода **0**. По целям: lib — 37,
lib test — 44 (cargo печатает дедуплицированно; 43 строки = 42 уникальных
сайта, один сайт напечатан дважды: `cli/init.rs:31:5`). Файлов — **17**,
крейтов — **1** (`vibe-index`). Предупреждений — 5 (неиспользуемые импорты
после удаления определений; `deny(warnings)` в крейте нет).

### (б) По коду ошибки

| код | сколько | что означает В ЭТОМ случае | образец |
|---|---|---|---|
| E0507 | 17 | значение `kind`/`naming` нельзя ни скопировать, ни вынести из-за `&` — потерян `Copy` | `index/inverted.rs:112:27` — «cannot move out of `entry.kind` which is behind a shared reference» |
| E0599 | 13 | пропали `repo_name`/`as_str`/`default`/`value_parser`; `PackageKind::all()` больше не разрешается | `cli/add.rs:165:23` — «no method named `repo_name` found for enum `…repomd::NamingConvention`» |
| E0382 | 5 | второе употребление перемещённого значения — тот же потерянный `Copy` | `cli/add.rs:101:9` — «use of moved value: `kind`» |
| E0117 | 3 | `Display`×2 и `FromStr`×1 на чужом типе — орфанное правило | `types/kinds.rs:42:1` — «only traits defined in the current crate can be implemented for types defined outside of the crate» |
| E0116 | 2 | inherent-impl на чужом типе (`impl PackageKind`, `impl NamingConvention`) | `types/kinds.rs:18:1` — «cannot define inherent `impl` for a type outside of the crate where the type is defined» |
| E0282 | 1 | `<PackageKind as FromStr>::from_str(...)` без законного impl — вывод типа сломан | `types/kinds.rs:108:59` — «type annotations needed» |
| E0277 | 1 | `NamingConvention: ValueEnum` не выполнен — clap-аргумент `--naming` | `cli/init.rs:34:29` — «the trait bound `…repomd::NamingConvention: ValueEnum` is not satisfied» |
| E0004 | 1 | match в `as_str` не покрывает новый вариант `Unknown(_)` | `types/kinds.rs:20:15` — «non-exhaustive patterns: `&…PackageKind::Unknown(_)` not covered» |

### (в) По ПРИЧИНЕ (сумма = 43)

1. **Потеря трейта — 26 сайтов.**
   - `Copy` у `PackageKind` и `NamingConvention`: 22 сайта (E0507×17 +
     E0382×5; из них `PackageKind` — 19, `NamingConvention` — 3:
     `index/memory.rs:295`, `scanner/org_walk.rs:209:56`, `cli/init.rs:72`).
   - `Default` у `NamingConvention`: 1 сайт (`types/kinds.rs:148`,
     тест `naming_convention_default_is_fqdn`).
   - `clap::ValueEnum` у `NamingConvention`: 3 сайта (`cli/init.rs:31` ×2
     печати одной строки, `cli/init.rs:34`).
2. **Смена ТИПА поля — 0 сайтов.** Словари не являются полями заменённых
   определений в этой пробе.
3. **Смена ФОРМЫ варианта — 0 сайтов** в строгом смысле (упаковка в newtype —
   это проба C); открытый словарь добавил вариант — см. причину 6.
4. **Орфанное правило и его следствия — 16 сайтов.** Прямые: E0116×2
   (inherent-impl'ы), E0117×3 (`Display`×2, `FromStr`). Следствия
   исчезновения методов из орфанных impl'ов: `repo_name`×6, `as_str`×2,
   `PackageKind::all()`×2 (отрисовано как «is not an iterator»:
   `cli/reindex.rs:438`, `types/kinds.rs:107`), E0282×1 (вывод
   `FromStr::Err`).
5. **Несовместимость модулей — 0 сайтов.** Оба словаря реэкспортированы
   каждый из одного модуля; встреча `types::kinds::PackageKind` со
   сгенерированным происходит только в пробе B.
6. **Прочее — 1 сайт:** E0004 — открытый словарь: у сгенерированного
   `PackageKind` есть вариант `Unknown(String)`, которого нет у
   рукописного; существующие match'и становятся неполными. Это не смена
   формы существующего варианта, а расширение множества.

**Итог: 26 + 0 + 0 + 16 + 0 + 1 = 43 ✓.**

### (г) Что осталось НЕИЗМЕРЕННЫМ

Проверены цели `vibe-index` **lib** (37 ошибок) и **lib test** (44):
они Red. НЕ проверены (юниты, зависящие от упавшей lib): **bin-цель**
(`src/main.rs`), **интеграционные тесты** (`tests/*.rs`, включая
`wire_parity_entry.rs`), бенчи/примеры (их у крейта нет); доктесты в
`cargo check` не входят вообще. Остальные 17 крейтов workspace'а
проверены и зелёные — **ни один из них не зависит от `vibe-index`**
(`grep -l "vibe-index" crates/*/Cargo.toml` → только сам `vibe-index`),
поэтому замкнутость радиуса — свойство графа зависимостей, а не везение.
Дополнительно: rustc подавляет часть follow-up ошибок внутри уже
ошибочных выражений, так что 43 — нижняя граница числа сломанных строк.

---

## ПРОБА B — запись и её проекции (`entry/**` → `entry::*` и `by_name::*`)

### (а) Сводка

`EXIT=101` (`/tmp/blast-b.exit`). Строк ошибок **126**, без кода **0**.
По целям: lib — 87, lib test — 121; уникальных сайтов 125 (один дубликат
печати: `index/memory.rs:186:36`). Файлов — **26**, крейтов — **1**.
Предупреждений — 13 (неиспользуемые импорты проб). Лимит ошибок rustc
(128) не достигнут.

### (б) По коду ошибки

| код | сколько | что означает В ЭТОМ случае | образец |
|---|---|---|---|
| E0599 | 54 | пропали `new`×15, `::default`×14, `sort_key`×8, `SCHEMA_VERSION`×8, `finalise`×7, `minimal`×2 — всё из орфанных impl'ов и потерянного `Default` | `index/memory.rs:123:27` — «no function or associated item named `new` found for struct `…by_name::PackageEntry`» |
| E0308 | 47 | несовпадение типов: 22× `PackageKind` (рукописный против сгенерированного), 8× `VersionEntry` (`entry` против `by_name`), 16× `Option<Подструктура>` против голой, 1× каскадный `String`/`&str` | `cli/add.rs:118:19` — «expected `Option<ProvidesEntry>`, found `ProvidesEntry`» |
| E0116 | 10 | inherent-impl'ы на чужих типах: `VersionEntry`, `PackageEntry`, `NameEntry`, `FeaturesEntry`, `I18nEntry` + 5× `is_empty` | `types/entry/relations.rs:18:1` |
| E0277 | 7 | `Display` отсутствует у сгенерированных словарей×4; итератор `Vec<&entry::VersionEntry>` из `&by_name::VersionEntry`×2; `Vec<entry::PackageKind>` из `types::kinds::PackageKind`×1 | `cli/dump.rs:46:17` — «`…entry::PackageKind` doesn't implement `std::fmt::Display`» |
| E0609 | 6 | обращения к полям `capabilities`/`min_vibe_version` через ставшее `Option<…>` поле | `index/inverted.rs:109:40` — «no field `capabilities` on type `Option<…ProvidesEntry>`» |
| E0271 | 2 | тип элемента итератора — `by_name::VersionEntry`, а ждут `entry::VersionEntry` | `index/memory.rs:186:36` — «…to be an iterator that yields `&VersionEntry`, but it yields `&VersionEntry`» |

### (в) По ПРИЧИНЕ (сумма = 126)

1. **Потеря трейта — 14 сайтов:** `Default` у семи подструктур
   (`CompatibilityEntry`, `ProvidesEntry`, `RequiresEntry`,
   `ObsoletesEntry`, `ConflictsEntry`, `FeaturesEntry`, `I18nEntry`):
   7 вызовов в `types/entry/mod.rs:83–91` (`minimal()`) + 7 в
   `types/entry/tests.rs:28–36`. `Copy` у `DeliveryMode` потерян, но
   **0** сломанных сайтов — по значению его никто не переиспользует.
2. **Смена ТИПА поля — 23 сайта:** семь полей записи
   (`compatibility`, `provides`, `requires`, `obsoletes`, `conflicts`,
   `features`, `i18n`) стали `Option<…>`: E0308-Option×16
   (`cli/add.rs:117–125`, `scanner/org_walk.rs:220–228`,
   `server/routes/packages.rs:178,292`, `cli/list.rs:69`,
   `cli/reindex.rs:447`, `index/search.rs:90`) + E0609×6 (обращения к
   полю через `Option`) + 1 каскадный E0308 `String`/`&str`
   (`index/inverted.rs:116` — вывод типа `cap` сломан E0609 на :109).
3. **Смена ФОРМЫ варианта — 0 сайтов.** Новый вариант `Unknown(String)`
   у `DeliveryMode`/`PackageKind` не сломал ни одного match'а —
   exhaustive-сопоставлений по этим словарям в крейте нет.
4. **Орфанное правило и его следствия — 54 сайта:** E0116×10 (inherent:
   `VersionEntry::{SCHEMA_VERSION, minimal, sort_key}`,
   `PackageEntry::{new, finalise}`, `NameEntry::{new, finalise}`,
   `FeaturesEntry::is_empty`, `I18nEntry::is_empty`, 5× relations
   `is_empty`) + пропавшие члены: `new`×15, `finalise`×7, `sort_key`×8,
   `minimal`×2, `SCHEMA_VERSION`×8 + `Display`×4 (E0277: impl из
   `kinds.rs` не переносится на сгенерированный тип — тот самый
   орфанный запрет в действии на дальней дистанции).
5. **Несовместимость модулей — 35 сайтов:** (i) рукописный
   `types::kinds::PackageKind` против сгенерированного
   `entry::PackageKind`/`by_name::PackageKind` — E0308×22 + E0277×1
   (`scanner/manifest.rs:80`: `Vec<entry::PackageKind>` нельзя собрать
   из `types::kinds::PackageKind`); (ii) **главный вопрос пробы B**:
   `by_name::PackageEntry::versions` типизирован
   `by_name::VersionEntry`, а не `entry::VersionEntry` — E0308×8 +
   E0271×2 + E0277×2 (`index/memory.rs:186`, `cli/get.rs:72,74`,
   `index/search.rs:135,165`, `journal/project.rs:193`,
   `server/routes/packages.rs:215`). Каждый сгенерированный модуль
   несёт СВОЮ копию словаря типов (JTD не умеет межфайловые ссылки), и
   «реэкспорт из entry» не склеивает копии.
6. **Прочее — 0 сайтов** (каскад `String`/`&str` отнесён к причине 2
   по корню).

**Итог: 14 + 23 + 0 + 54 + 35 + 0 = 126 ✓.**

### (г) Что осталось НЕИЗМЕРЕННЫМ

Как в пробе A: проверены lib (87) и lib test (121); bin-цель и
интеграционные тесты (`tests/*.rs`, включая `wire_parity_entry.rs`,
который ходит в `vibe-wire` напрямую) до проверки не дошли. Каскадное
подавление follow-up ошибок: в функциях, где упало обращение к полю
через `Option`, дальнейшие ошибки того же выражения подавлены — 126
есть нижняя граница.

---

## ПРОБА C — манифест каталога (`repomd.rs` → `repomd::{Repomd, RepomdFileEntry}`)

### (а) Сводка

`EXIT=101` (`/tmp/blast-c.exit`). Строк ошибок **40**, без кода **0**.
По целям: lib — 17, lib test — 29; уникальных сайтов (код+место) — 29
(11 сайтов напечатаны дважды — lib и lib test с разной полнотой имён
типов). Файлов — **5** (`types/repomd.rs`, `index/repomd.rs`,
`index/memory.rs`, `index/memory/tests.rs`, `cli/verify.rs`), крейтов —
**1**. Предупреждений — 9.

### (б) По коду ошибки

| код | сколько | что означает В ЭТОМ случае | образец |
|---|---|---|---|
| E0599 | 24 | пропали конструкторы `::file`×14 / `::directory`×7 и `SCHEMA_VERSION`×3 | `index/memory.rs:221:30` — «no variant or associated item named `file` found for enum `RepomdFileEntry`» |
| E0559 | 6 | конструкторы `directory()`/`file()` пишут вариант как структурный, а он теперь newtype с боксом | `types/repomd.rs:23:38` — «variant `RepomdFileEntry::Directory` has no field named `entries`» |
| E0308 | 4 | `naming` в `Repomd` ждёт сгенерированный `repomd::NamingConvention`, приходит рукописный `types::kinds::…` (и один раз наоборот) | `index/memory.rs:295:21` — «expected `NamingConvention`, found `types::kinds::NamingConvention`» |
| E0769 | 2 | потребитель строит `File { size, sha256 }`, а вариант — newtype | `cli/verify.rs:80:13` — «tuple variant `RepomdFileEntry::File` written as struct variant» |
| E0116 | 2 | inherent-impl'ы: `impl Repomd` (SCHEMA_VERSION), `impl RepomdFileEntry` (конструкторы) | `types/repomd.rs:17:1` |
| E0026 | 2 | match-плечо `Directory { entries }` — у newtype-варианта нет такого поля | `cli/verify.rs:102:42` — «variant `Directory` does not have a field named `entries`» |

### (в) По ПРИЧИНЕ (сумма = 40)

1. **Потеря трейта — 0 сайтов.** Рукописные derive у `Repomd` и
   `RepomdFileEntry` (`Debug, Clone, PartialEq, Eq, Serialize,
   Deserialize`) полностью совпадают с сгенерированными — терять
   нечего.
2. **Смена ТИПА поля — 0 прямых сайтов.** `size: u64 → u32` у
   файлового плеча **не дал ни одной собственной ошибки**: все
   потенциальные сайты употребления `size` перекрыты ошибками формы
   (E0559/E0769/E0026) и потерей конструкторов (E0599). Сужение
   неизмеримо в этой пробе напрямую — оно выстрелит только после
   починки формы. `naming` сменил тип — но это причина 5.
3. **Смена ФОРМЫ варианта — 10 строк (5 уникальных сайтов):**
   рукописное `Directory { entries }` / `File { size, sha256 }` против
   сгенерированного `Directory(Box<RepomdFileEntryDirectory>)` /
   `File(Box<RepomdFileEntryFile>)`. Сайты: тела конструкторов
   `types/repomd.rs:23,28,29`; потребитель `cli/verify.rs:80`
   (E0769) и match-плечи `cli/verify.rs:102` + `types/repomd.rs:105`
   (E0026); тестовое `types/repomd.rs:98` (E0769). Match-плечи и
   построения затронуты в двух файлах (`types/repomd.rs`,
   `cli/verify.rs`).
4. **Орфанное правило и его следствия — 26 строк:** E0116×2 + пропавшие
   `file`/`directory`×21 и `SCHEMA_VERSION`×3.
5. **Несовместимость модулей — 4 сайта:** поле `Repomd.naming`
   типизировано сгенерированным `repomd::NamingConvention`, а код
   передаёт рукописный `types::kinds::NamingConvention` (и в
   `index/memory.rs:357` — наоборот): `index/memory.rs:295,357`,
   `index/repomd.rs:63`, `types/repomd.rs:50`. Это стык пробы A и C:
   словарь остался рукописным, манифест — сгенерированный.
6. **Прочее — 0.**

**Итог: 0 + 0 + 10 + 26 + 4 + 0 = 40 ✓.**

### (г) Что осталось НЕИЗМЕРЕННЫМ

Как в A/B: bin и интеграционные тесты не дошли до проверки. Специфично
для C: (i) след `u64→u32` замаскирован (см. причину 2) — прямой цены
этой смены пробы C не назвали; (ii) `RepomdFileEntry` перевёрстан в
тегированное объединение — serde-семантика (адъютор `tag = "kind"`,
renames) в `cargo check` не проверяется вообще: round-trip-тесты
остались за бортом всех трёх проб.

---

## §5. Инвентарь дельты (read-only)

Рукописных типов в `crates/vibe-index/src/types/**` — **19**. Общее у
всех: сгенерированная сторона выводит `Debug, Clone, PartialEq, Eq,
Serialize, Deserialize` (у словарей Ser/De — вручную, из-за открытого
`Unknown(String)`). Совпадающее — не теряется; в таблице только дельта.

| тип | derive рукописный | derive сгенерированный | что ТЕРЯЕТСЯ | что МЕНЯЕТСЯ в полях | где живёт сгенерированный |
|---|---|---|---|---|---|
| `PackageKind` | D,Cl,**Copy**,PE,E,**PO,O,H,ValueEnum**,S,De | D,Cl,PE,E,S,De | **Copy, PartialOrd, Ord, Hash, ValueEnum** | + вариант `Unknown(String)`; порядок вариантов алфавитный | `…index::e1::entry` (+ копии в `by_name` и др. сгенерированных модулях) |
| `NamingConvention` | D,Cl,**Copy,Default**,PE,E,**ValueEnum**,S,De | D,Cl,PE,E,S,De | **Copy, Default, ValueEnum** | — | `…index::e1::repomd` |
| `CompatibilityEntry` | D,Cl,**Default**,PE,E,S,De | D,Cl,PE,E,S,De | **Default** | — (поля те же) | `…index::e1::entry` |
| `ProvidesEntry` | D,Cl,**Default**,PE,E,S,De | D,Cl,PE,E,S,De | **Default** | — | `…index::e1::entry` |
| `RequiresEntry` | D,Cl,**Default**,PE,E,S,De | D,Cl,PE,E,S,De | **Default** | — (порядок полей) | `…index::e1::entry` |
| `RequiresAnyEntry` | D,Cl,PE,E,S,De | D,Cl,PE,E,S,De | — | — | `…index::e1::entry` |
| `ObsoletesEntry` | D,Cl,**Default**,PE,E,S,De | D,Cl,PE,E,S,De | **Default** | — | `…index::e1::entry` |
| `ConflictsEntry` | D,Cl,**Default**,PE,E,S,De | D,Cl,PE,E,S,De | **Default** | — | `…index::e1::entry` |
| `WorkspaceOriginEntry` | D,Cl,PE,E,S,De | D,Cl,PE,E,S,De | — | — (порядок полей) | `…index::e1::entry` |
| `FeaturesEntry` | D,Cl,**Default**,PE,E,S,De | D,Cl,PE,E,S,De | **Default** | — | `…index::e1::entry` |
| `DeliveryMode` | D,Cl,**Copy**,PE,E,S,De | D,Cl,PE,E,S,De | **Copy** | + вариант `Unknown(String)` | `…index::e1::entry` |
| `SubskillEntry` | D,Cl,PE,E,S,De | D,Cl,PE,E,S,De | — | — (порядок полей) | `…index::e1::entry` |
| `I18nEntry` | D,Cl,**Default**,PE,E,S,De | D,Cl,PE,E,S,De | **Default** | — | `…index::e1::entry` |
| `BootSnippetEntry` | D,Cl,PE,E,S,De | D,Cl,PE,E,S,De | — | — | `…index::e1::entry` |
| `VersionEntry` | D,Cl,PE,E,S,De (+`#[spec]`) | D,Cl,PE,E,S,De | — (трейтов), но `#[spec]`-атрибут исчезает с типом | **7 полей стали `Option<…>`: compatibility, provides, requires, obsoletes, conflicts, features, i18n** | `…index::e1::entry` |
| `PackageEntry` | D,Cl,PE,E,S,De (+`#[spec]`) | D,Cl,PE,E,S,De | — | `versions: Vec<entry::VersionEntry>` → `Vec<by_name::VersionEntry>` (другой модуль!) | `…index::e1::by_name` |
| `NameEntry` | D,Cl,PE,E,S,De (+`#[spec]`) | D,Cl,PE,E,S,De | — | имя корня в схеме — `ByName`; `packages` — из `by_name` | `…index::e1::by_name` (как `ByName`) |
| `Tombstone` | D,Cl,PE,E,S,De | D,Cl,PE,E,S,De | — | — | `…index::e1::by_name` |
| `Repomd` | D,Cl,PE,E,S,De (+`#[spec]`) | D,Cl,PE,E,S,De | — | `naming` — сгенерированный `repomd::NamingConvention` вместо рукописного | `…index::e1::repomd` |
| `RepomdFileEntry` | D,Cl,PE,E,S,De | D,Cl,PE,E,S,De | — | **форма вариантов**: структурные → newtype с `Box`; `size: u64 → u32` | `…index::e1::repomd` |

*(D=Debug, Cl=Clone, PE=PartialEq, E=Eq, PO=PartialOrd, O=Ord, H=Hash,
S=Serialize, De=Deserialize.)*

### Три числа

**1. Сайты `::default()` в `crates/vibe-index/src` — всего и на теряющих
`Default`:**

```sh
grep -rn "::default()" crates/vibe-index/src --include="*.rs" | wc -l          # → 48
grep -rhoE "(CompatibilityEntry|ProvidesEntry|RequiresEntry|ObsoletesEntry|ConflictsEntry|FeaturesEntry|I18nEntry|NamingConvention)::default\(\)" \
  crates/vibe-index/src --include="*.rs" | wc -l                               # → 15
```

**48 всего, 15 — на типах, теряющих `Default`** (7 в `types/entry/mod.rs`
в `minimal()`, 7 в `types/entry/tests.rs`, 1 `NamingConvention` в
`types/kinds.rs:148`). Сверка с компилятором: проба A дала 1 ошибку
`default` (NamingConvention), проба B — 14 (те самые 7+7); **1+14 = 15 =
греп-оценка. Расхождений нет.**

**2. Сайты `PackageKind` там, где нужен `Ord`/`Hash`:**

```sh
grep -rnE "(BTreeMap|HashMap|BTreeSet|HashSet)<[^>;]*PackageKind" crates/ --include="*.rs" \
  | grep -v crates/vibe-wire | grep -v crates/vibe-core                          # → 3 сайта, все в vibe-cli
grep -rniE "kind(s)?\.(sort|cmp)|sort.*kind|\.cmp\(&.*kind" crates/ --include="*.rs" ...  # → 2 cmp-сайта в vibe-cli, 1 sort в vibe-settings
```

Греп-оценка: **0 сайтов в радиусе.** Найденные 5 (+1 sort) — не наш
словарь: `vibe-cli` не зависит от `vibe-index` и употребляет
**`vibe-core`'s** `PackageKind` (третья копия), а `kinds.sort()` в
`vibe-settings` сортирует вид диагностики. Компилятор согласен: **в пробе
A — 0 ошибок на `Ord`/`Hash`/`PartialOrd`-границах.** Расхождение с
прозой дерева: комментарий `scanner/manifest.rs:14` называет `Ord`
причиной дублирования словаря («needing the `Ord` + `clap::ValueEnum`»),
но измерение показывает: в радиусе замены `Ord`/`Hash` не нужны ни одному
сайту — трейт-этаж держится предположением, а не употреблением.

**3. Сайты `PackageKind`/`DeliveryMode` по значению (потеря `Copy`):**

```sh
grep -rnE "\( *([a-z_]+): *PackageKind[,)]|-> *PackageKind" crates/vibe-index/src --include="*.rs" \
  | grep -v "&PackageKind" | grep -v "PackageKind::"                            # → 3 сигнатуры
grep -rnE "\( *([a-z_]+): *DeliveryMode[,)]|-> *DeliveryMode" crates/vibe-index/src --include="*.rs" # → 1 сигнатура
```

Греп-оценка: `PackageKind` — 3 сигнатуры-кандидата (плюс ~98 шумных
упоминаний `kind` грепом не разделяются на копию/ссылку); `DeliveryMode`
— 1 кандидат (`scanner/manifest.rs:183`, конструктор из
`vibe-core`-значения). **Компилятор точнее и называет расхождение:**
проба A сломала **19** сайтов `PackageKind`-по-значению (17×E0507 + 5×E0382
= 22 Copy-сайта, минус 3 `NamingConvention`) — греп по сигнатурам видит
только 3 из 19, потому что ломаются не сигнатуры, а места передачи и
повторного употребления (`e.kind` в замыкании, `v.kind` из-за `&`,
второй проход по `kind`); `DeliveryMode` — **0** сломанных (единственный
кандидат строит свежее значение). Доверюсь компилятору: **19 и 0**.

---

## Deviations (§0.9 — расхождения числа пакета/прозы с деревом)

1. **`by_name` не содержит `NameEntry`.** Пакет: «эти три живут не в
   модуле `entry`, а в `…by_name`. Возьми их оттуда». Дерево: корень
   `by_name`-схемы называется **`ByName`** (`by_name/mod.rs:12`); взято
   `pub use …by_name::ByName as NameEntry;`. Вердикт компилятора это
   подтверждает: ошибки зовут тип `struct ByName`.
2. **Перенос `vibe-wire` в `[dependencies]` нужен каждой пробе**, не
   только A (пакет описал его в §3.1): реэкспорт в lib-коде не
   разрешается из `[dev-dependencies]`. Применён во всех трёх пробах,
   каждый раз возвращён.
3. **Прежняя оценка чтением (f42c-reexport-radius.md) устарела в
   главном:** она утверждает «сгенерированные выводят только
   Serialize, Deserialize — 74 из 74». Сегодняшнее дерево выводит
   `Debug, Clone, PartialEq, Eq` тоже — значит `PartialEq`/`Debug`/
   `Clone` НЕ теряются, и «трейт-этаж исчезает» верен только для
   `Copy/PartialOrd/Ord/Hash/Default/ValueEnum`. Пакет был прав:
   чтение систематически недосчитывается.
4. **Список ожидаемых кодов в пакете — иллюстрация, не список** (§0.9):
   фактически правили баллом E0116 (inherent-impl — его нет в списке),
   E0599, E0609, E0271, E0559, E0769, E0026, E0282; E0369/E0119 не
   встретились ни разу.
5. `types/mod.rs` (пункт 9 списка §2) ни одна проба не потребовала —
   не правился ни разу.

## Как воспроизвести

```sh
# разогрев (зелёный) + калибровка счётчика
cargo check -p vibe-index --all-targets > /tmp/blast-warm.txt 2>&1; echo "EXIT=$?"
grep -E "^error(\[E[0-9]+\])?:" /tmp/blast-warm.txt | grep -vE "aborting due to|could not compile" | wc -l   # 0

# каждая проба: заменить определения на pub use (файлы §2 пакета F42C3-BLAST),
# перенести vibe-wire из [dev-dependencies] в [dependencies], затем:
cargo check --workspace --all-targets > /tmp/blast-X.txt 2>&1; echo "EXIT=$?"   # 101 (X = a|b|c)

# числа находки
grep -E "^error(\[E[0-9]+\])?:" /tmp/blast-X.txt | grep -vE "aborting due to|could not compile" | wc -l   # 43 / 126 / 40
grep -E "^error(\[E[0-9]+\])?:" /tmp/blast-X.txt | grep -vE "aborting due to|could not compile" \
  | grep -oE "^error(\[E[0-9]+\])?" | sort | uniq -c | sort -rn
grep -E "aborting due to|could not compile" /tmp/blast-X.txt        # счётчики по целям (37/44, 87/121, 17/29)
# файлы/крейты прогона:
awk '/^error(\[E[0-9]+\])?:/ && !/aborting due to|could not compile/ {e=1; next} e && /--> / {sub(/^ +--> /,""); sub(/:[0-9]+:[0-9]+$/,""); print; e=0}' /tmp/blast-X.txt | sort -u

# три числа §5 — команды напечатаны в их разделе выше
```

Проверка возврата файлов: `diff` против сохранённой копии из
системного temp (не git) — все 8 файлов §2 байтово идентичны исходным;
`md5sum Cargo.lock` не изменился (`fcf92cae…` до и после всех проб);
контрольный `cargo check -p vibe-index --all-targets` после откатов —
`EXIT=0`.
