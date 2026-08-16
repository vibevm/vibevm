# F42-RADIUS — радиус семи оставшихся преобразований пост-обработки

**Чем мерил:** только чтение дерева — Read/Glob/Grep плюс читающие
shell-команды (`grep -E`, `wc -l`, `find`, `ls`, `awk`-разбор пар
`rename`↔идентификатор). Worktree `wt/F42-RADIUS`, замер от корня рабочего
дерева. Каждое число несёт команду или `файл:строку`.

**Что НЕ запускалось:** ни одной команды `git` (запрещена пакетом), ни одной
команды `cargo` (запрещена пакетом), генератор не запускался. Замер
статический: схемы прочитаны целиком (13 хостовых + словник + схема движка),
оба сгенерированных дерева прочитаны выборочно целиком (entry, by_name,
journal, repomd, format_id) и по греп-срезам (остальные листы).

**Дата:** 2026-08-16.

---

## 1. ВЕРДИКТ

**1. Сколько сайтов двигает каждое из семи оставшихся преобразований.**
Нумерация следует пакету: №1 словарь, №2 snake_case, №3 пустое, №4 реестр
(deny_unknown_fields), №5 умолчание скаляра, №6 DateTime, №8 доменные типы;
№7 (боксирование) построено (`xtask/src/codegen/postproc.rs`, все плечи
объединений уже `Box<…>`, например `crates/vibe-wire/src/generated/journal/e1/journal/mod.rs:91-121`).

| № | преобразование | сайтов | в каких файлах |
|---|---|---|---|
| 1 | открытие словаря по `x-vocabulary` | **14** JTD-enum-копий (10 станут открытыми, 3 останутся закрытыми, 1 без аннотации) + **4** не-JTD enum'а, которые трогать НЕ надо | 9 листов generated: by_cap, by_name, by_purl, entry, repomd, init_report, journal, list_report, registry_sync_report |
| 2 | camelCase → snake_case | **81** поле | 10 файлов: by_name 21, journal 21, entry 17, list_report 6, repomd 5, registry_publish_report 5, install_plan 3, install_report 1, by_purl 1, uninstall_report 1 (команда в §5) |
| 3 | политика пустого по `x-empty` | **31** сайт коллекций (15 в схемах + 16 в словнике); аннотировано сегодня 21, долг 10 | 10 файлов схем + `formats/vocabularies.json` |
| 4 | `deny_unknown_fields` по реестру | **69** структур хоста (сегодня атрибут встречается **0** раз во всём дереве) | все 13 листов generated |
| 5 | `Option<Box<скаляр>>` → скаляр с умолчанием | **48** полей, из них **6** с `x-default` (yanked/frozen × 3 копии) и **42** без | 5 файлов: by_name 15, journal 14, entry 13, list_report 5, install_plan 1 |
| 6 | `DateTime<Utc>` | **7** полей `: Timestamp` + **4** алиаса `pub type Timestamp = String` | by_name (3 поля), journal (2), entry (1), repomd (1) |
| 8 | доменные типы по `x-rust-type` | **15** алиасов (`Group`×5, `Version`×5, `Timestamp`×4, `Entry`×1) + 9 полей `: Version` + 13 полей `: Group`; плюс 4 аннотации, не отражённые генератором | 5 файлов: by_cap, by_name, by_purl, entry, journal (+ repomd алиас Timestamp) |

**Преобразований с нулём сайтов нет.** Но радиус *вне* `generated/**` у трёх
из них — **ноль**: №2, №5 и №8 не называет ни один потребитель (§8):
единственный продуктовый потребитель строит `init_report` (там нет ни одного
camelCase-поля, ни одного `Option<Box<>>`, ни одного алиаса), оракулы
сравнивают `serde_json::Value` и не трогают идентификаторы полей вообще.

**2. Долг аннотаций для правила «нет аннотации — ошибка генерации».**
Хостовый дом: **30 сайтов** — 1 `x-vocabulary` (init_report `outcome_action`),
10 `x-empty` (шесть CLI-отчётов), 19 `x-default` (опциональные скаляры без
умолчания, полный список в §3). По словнику долг `x-default` — 11 из 13.
Если правило применять глобально, оно краснеет ещё на **15 сайтах движка**
(4 enum + 5 коллекций + 6 опциональных скаляров, все без аннотаций, §10) —
дом, который кампании запрещено трогать.

**3. Что опровергает записанное в базовой линии.** **Ничего — все восемь
утверждений U1–U8 подтверждены** (§2). Опровергнуты два ожидания, стоявшие
рядом с линией, но не в ней:

- `#![allow(non_snake_case)]` стоит **не в сгенерированных файлах**, а ровно
  один раз — в рукописном `crates/vibe-wire/src/lib.rs:52`. Чем меняет шаг:
  преобразование №2 снимает не по-файловый allow, а один crate-wide, и
  снимается он правкой рукописного lib.rs, не постпроцессором.
- «13 схем хостового дома» — верно, но `formats/REGISTRY.toml:195` ссылается
  на четырнадцатую, `schemas/hello/e1/hello.jtd.json`, которой **нет на
  диске** (`find schemas -type f` — 13 файлов). Чем меняет шаг: преобразование
  №4 (реестр) встретит формат `handshake` с несуществующей схемой — гейт
  должен либо отказаться громко, либо это отдельная дыра (§10).

---

## 2. Подтверждение-или-опровержение записанного (U1–U8)

- **U1 ПОДТВЕРЖДЕНО.** `crates/vibe-cli/Cargo.toml:31` — `vibe-wire.workspace = true`
  в секции `[dependencies]` (секция открывается строкой 17). 
  `crates/vibe-index/Cargo.toml:55` — `vibe-wire = { workspace = true }` в
  секции `[dev-dependencies]` (секция открывается строкой 44), с комментарием
  в строках 51-54: «Dev-only on purpose: until F4.2 replaces the hand-written
  types with re-exports of the generated ones, the library itself keeps no
  runtime edge on vibe-wire».
- **U2 ПОДТВЕРЖДЕНО.** Единственный продуктовый `use vibe_wire` — 
  `crates/vibe-cli/src/commands/init/helpers.rs:94`:
  `use vibe_wire::generated::init_report::{InitReport, Outcome as WireOutcome, OutcomeAction};`.
  Остальные использования — пять тестов `crates/vibe-index/tests/wire_parity_*.rs`
  и путевые строки в `xtask/src/codegen/mod.rs:85,240,371,426` (механизм
  сборки, типы не импортирует). Команда: `grep -rn "vibe_wire" crates/ xtask/ --include="*.rs" | grep -v "crates/vibe-wire/"`.
- **U3 ПОДТВЕРЖДЕНО.** `grep -rE "pub [a-z]+[A-Z][A-Za-z0-9]*:" crates/vibe-wire/src/generated/ | wc -l` → **81**.
  Разбивка: by_name 21, journal 21, entry 17, list_report 6, repomd 5,
  registry_publish_report 5, install_plan 3, install_report 1, by_purl 1,
  uninstall_report 1.
- **U4 ПОДТВЕРЖДЕНО.** `grep -rc "Option<Box<" …` → всего **121**; по файлам:
  by_name 40, entry 37, journal 38, list_report 5, install_plan 1 — в точности
  записанная разбивка. Классы: коллекции 45, скаляры 48, структуры 28 (§6).
- **U5 ПОДТВЕРЖДЕНО.** `grep -rn "x-default" schemas/ formats/` → ровно два
  вхождения, оба `formats/vocabularies.json:410` и `:416`, оба
  `"x-default": false` (поля `yanked` и `frozen` фрагмента `version_entry`).
- **U6 ПОДТВЕРЖДЕНО.** `grep -rn "\"type\": \"timestamp\"" schemas/ formats/ packages/org.vibevm.ai-native/core-ai-native/v0.8.0/schemas/` →
  пусто (exit 1). Время на проводе — всегда `ref` на фрагмент `timestamp`
  (5 сайтов: by_name:11,41; repomd:23; journal:9; vocabularies.json:318), сам
  фрагмент объявлен `"type": "string"` (`formats/vocabularies.json:38-43`).
- **U7 ПОДТВЕРЖДЕНО.** В `formats/REGISTRY.toml` ровно одна запись несёт
  `foreign_parsers = "none"` — строка 180, `[format.config]`, и у неё же
  `schema = "none"` (строка 178). Ни у одного формата с построенной JTD-схемой
  роль не «none» (все JTD-форматы — `many` либо `ours`).
- **U8 ПОДТВЕРЖДЕНО.** `grep -cn "x-" packages/org.vibevm.ai-native/core-ai-native/v0.8.0/schemas/specmap.jtd.json` → **0**
  (exit 1). Схема движка не несёт ни одной `x-…`-аннотации.

---

## 3. Долг аннотаций — по схеме, по сайту

Столбец «коллекций» считает САЙТЫ (`elements`/`values`), не сырые вхождения
ключей: в словнике grep даёт 18 сырых вхождений, но два сайта
`features`/`exclusive` — это `values` поверх `elements` и несут оба ключа,
поэтому сайтов 16. «Опциональный скаляр» — член `optionalProperties` с типом
string/boolean/числовой (или `ref` на фрагмент-скаляр словника — такие
посчитаны скалярами и помечены ниже). «Строка-доменный-тип» — именованное
определение `"type": "string"` (кандидат `x-rust-type`); inline-строковые
члены (name, path, reason…) НЕ считаются: их рукописные двойники остаются
`String`, доменными типами они не объявлены.

| схема | enum'ов | из них с x-vocabulary | коллекций | из них с x-empty | опц. скаляров | из них с x-default | строк-доменных-типов | из них с x-rust-type |
|---|---|---|---|---|---|---|---|---|
| init_report | 1 | 0 | 1 | 0 | 0 | 0 | 0 | 0 |
| install_plan | 0 | 0 | 2 | 0 | 1 | 0 | 0 | 0 |
| install_report | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 |
| list_report | 0 | 0 | 2 | 0 | 4 | 0 | 0 | 0 |
| registry_publish_report | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| registry_sync_report | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 |
| uninstall_report | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 |
| index/e1/entry | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| index/e1/by_name | 0 | 0 | 2 | 2 | 2 | 0 | 0 | 0 |
| index/e1/by_cap | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| index/e1/by_purl | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 0* |
| index/e1/repomd | 0 | 0 | 1 | 1 | 0 | 0 | 0 | 0 |
| journal/e1/journal | 0 | 0 | 2 | 2 | 1 | 0 | 0 | 0* |
| **formats/vocabularies.json** | 3 | 3 | 16 | 16 | 13 | 2 | 3 | 3 |
| **движок specmap.jtd.json** | 4 | 0 | 5 | 0 | 6 | 0 | 0 | 0 |

\* `x-rust-type` в этих двух схемах стоит на enum'ах (`BindingSite`,
`Event`) и корне (`JournalRecord`) — не на строковых типах; они разобраны в §7.

Итого по хостовому дому: enum-сайтов 5 (4 аннотированы), коллекций 31
(аннотировано 21), опциональных скаляров 21 (аннотировано 2), строк-доменных-типов 3 (аннотированы все 3).

**Поимённый список сайтов БЕЗ нужной аннотации (хостовый дом, 30):**

`x-vocabulary` (1):
1. `schemas/init_report.jtd.json` → `definitions.outcome_action` — enum
   `["created","kept"]` (строки 49-54) без аннотации.

`x-empty` (10):
2. `schemas/init_report.jtd.json` → `properties.outcomes`
3. `schemas/install_plan.jtd.json` → `properties.plans`
4. `schemas/install_plan.jtd.json` → `definitions.plan_entry.properties.writes`
5. `schemas/install_report.jtd.json` → `properties.installed`
6. `schemas/install_report.jtd.json` → `definitions.applied_report.properties.paths`
7. `schemas/list_report.jtd.json` → `properties.packages`
8. `schemas/list_report.jtd.json` → `definitions.list_entry.properties.files_written`
9. `schemas/registry_sync_report.jtd.json` → `properties.refreshed`
10. `schemas/registry_sync_report.jtd.json` → `properties.skipped`
11. `schemas/uninstall_report.jtd.json` → `properties.paths`

`x-default` (19; `→` после тире — как поле объявлено в сгенерированном
выходе, все они сегодня `Option<Box<…>>`):
12. `schemas/install_plan.jtd.json` → `definitions.plan_entry.optionalProperties.boot_snippet` — `Option<Box<String>>`
13. `schemas/list_report.jtd.json` → `definitions.list_entry.optionalProperties.registry` — `Option<Box<String>>`
14. `…list_report… → …optionalProperties.source_ref` — `Option<Box<String>>`
15. `…list_report… → …optionalProperties.resolved_commit` — `Option<Box<String>>`
16. `…list_report… → …optionalProperties.overridden` — `Option<Box<bool>>`
17. `schemas/index/e1/by_name.jtd.json` → `definitions.package_entry.optionalProperties.latest_stable` — `Option<Box<Version>>` (**ref на фрагмент-скаляр словника, посчитан скаляром**)
18. `…by_name… → definitions.tombstone.optionalProperties.superseded_by` — `Option<Box<String>>`
19. `schemas/journal/e1/journal.jtd.json` → `definitions.event.mapping.removed.optionalProperties.version` — `Option<Box<Version>>` (**тоже ref-скаляр**; см. §9-смежное примечание схемы о `"version": null`)
20. `formats/vocabularies.json` → `boot_snippet_entry.optionalProperties.category`
21. `…vocab… → compatibility_entry.optionalProperties.min_vibe_version`
22. `…vocab… → i18n_entry.optionalProperties.default`
23. `…vocab… → subskill_entry.optionalProperties.describes`
24. `…vocab… → subskill_entry.optionalProperties.description`
25. `…vocab… → workspace_origin_entry.optionalProperties.commit`
26. `…vocab… → version_entry.optionalProperties.resolved_commit`
27. `…vocab… → version_entry.optionalProperties.license`
28. `…vocab… → version_entry.optionalProperties.description`
29. `…vocab… → version_entry.optionalProperties.homepage`
30. `…vocab… → version_entry.optionalProperties.describes`

Не-скалярные опционалы, НЕ попавшие в долг по этой оси (для полноты):
`tombstone` и семь вложенных структур `version_entry` (§9) — это `ref` на
структуры, не скаляры; `by_name.optionalProperties.tombstone`, 
`version_entry.optionalProperties.{workspace_origin,compatibility,provides,requires,obsoletes,conflicts,features,i18n,boot_snippet}`.

---

## 4. Радиус преобразования №1 (открытие словаря)

Все `pub enum` хостового дерева — **18** (команда
`grep -rn "pub enum" crates/vibe-wire/src/generated/`; число вариантов —
awk-разбор тел, команда в §11):

| enum | файл:строка | вариантов | откуда | x-vocabulary |
|---|---|---|---|---|
| PackageKind | by_cap/mod.rs:37 | 6 | фрагмент `package_kind` | **open** (`vocabularies.json:4`) |
| PackageKind | by_name/mod.rs:146 | 6 | он же | **open** |
| PackageKind | by_purl/mod.rs:55 | 6 | он же | **open** |
| PackageKind | entry/mod.rs:102 | 6 | он же | **open** |
| PackageKind | journal/mod.rs:386 | 6 | он же | **open** |
| PackageKind | list_report/mod.rs:78 | 6 | он же | **open** |
| PackageKind | registry_sync_report/mod.rs:27 | 6 | он же | **open** |
| DeliveryMode | by_name/mod.rs:69 | 3 | фрагмент `delivery_mode` | **open** (`vocabularies.json:11`) |
| DeliveryMode | entry/mod.rs:51 | 3 | он же | **open** |
| DeliveryMode | journal/mod.rs:68 | 3 | он же | **open** |
| NamingConvention | repomd/mod.rs:56 | 4 | фрагмент `naming_convention` | **closed** (`vocabularies.json:18`) |
| NamingConvention | journal/mod.rs:361 | 4 | он же | **closed** |
| BindingSite | by_purl/mod.rs:40 | 2 | def `binding_site` | **closed** (`by_purl.jtd.json:32`) |
| OutcomeAction | init_report/mod.rs:52 | 2 | def `outcome_action` | **аннотации нет** |
| Event | journal/mod.rs:89 | 11 | def `event`, discriminator | не словарь — tagged union |
| RepomdFileEntry | repomd/mod.rs:77 | 2 | def `repomd_file_entry`, discriminator | не словарь — tagged union |
| FormatId | format_id/mod.rs:21 | 20 | **REGISTRY.toml, не JTD** | неприменимо |
| ForeignParsers | format_id/mod.rs:47 | 3 | **REGISTRY.toml, не JTD** | неприменимо |

**Не приходят из JTD и трогать их №1 не должен:** `FormatId` и
`ForeignParsers` — ветка TOML-реестра (`format_id/mod.rs:1`: «Generated by
`cargo xtask codegen` from `formats/REGISTRY.toml`»), они вообще не несут
`Serialize/Deserialize` (там же, строки 8-11). `Event` и `RepomdFileEntry`
приходят из JTD, но из `discriminator`-объединений — это не словари, и
«открытие» для них означало бы молча принять неизвестный тег, что прямо
противоречит их определению в схеме («An arm's `kind` outside this mapping
is rejected by the generated reader», `journal.jtd.json:27`).

**Ответ на ключевой вопрос:** открытыми (`Unknown(String)`) станут
**10** enum-копий (PackageKind ×7, DeliveryMode ×3); закрытыми останутся
**3** (NamingConvention ×2, BindingSite ×1); ещё **1** (OutcomeAction) —
сайт долга: без записи `x-vocabulary` в `schemas/init_report.jtd.json`
правило «нет аннотации — ошибка» падает на первом же прогоне. Списки —
таблица выше.

Смежное (за периметром хостового дома): у движка те же классы — 4 enum'а
(`EdgeProvenance`, `EdgeVerb`, `SpecUnitKind`, `SpecUnitStatus` в
`…/core-ai-native-specmap/src/generated/specmap/mod.rs:72,84,139,155`), все
без аннотаций, и крейт движка кампании запрещён.

---

## 5. Радиус преобразования №2 (camelCase → snake_case)

**Хостовое дерево: 81 поле** несёт camelCase-идентификатор при
snake_case-строке провода (U3; полный построчный список получен командой в
§11). Разбивка по файлам:

```
by_name 21, journal 21, entry 17, list_report 6, repomd 5,
registry_publish_report 5, install_plan 3, install_report 1,
by_purl 1, uninstall_report 1
```

**Выход движка — для сведения: 12** таких полей
(`grep -rcE "pub [a-z]+[A-Z][A-Za-z0-9]*:" packages/…/core-ai-native-specmap/src/generated/`
→ только specmap/mod.rs: 12). Числа двух домов не складываются: у движка
свой дом и запрет на правку.

**Где стоит `#![allow(non_snake_case)]`:** в сгенерированных файлах —
**нигде** (`grep -rn "non_snake_case" crates/vibe-wire/src/generated/ …` →
пусто). Единственное место — рукописный `crates/vibe-wire/src/lib.rs:52`:

```rust
// jtd-codegen 0.4.1 emits structs with `pub camelCase` field names
// (e.g. `removedCount`) when the JTD schema property is `removed_count`.
// …
#![allow(non_snake_case)]
```

(в `generated/format_id/mod.rs:13` стоит `#![allow(clippy::match_same_arms)]` —
другая lint, к №2 отношения не имеет). После №2 этот crate-wide allow можно
снять — но это правка lib.rs, не постпроцессора.

> **ПОПРАВКА 2026-08-17 — три утверждения ниже ЛОЖНЫ, и следующий читатель
> не должен по ним строить.** Awk-скрипт §11, на котором они стоят, **не может
> сработать ни на одном поле**: его шаблон `pub [A-Za-z]+[A-Za-z0-9]*: {0,1}$`
> требует конца строки сразу после двоеточия, а эмиссия всегда несёт тип и
> запятую (`pub contentHash: String,`). Проверено прямо: скрипту скормили поле,
> у которого провод намеренно сделан ДРУГИМ (`rename = "WRONG"`), — он
> промолчал. Пустой вывод был не доказательством нуля исключений, а отсутствием
> измерения.
>
> Исключение существует, и оно ровно одно:
> `schemas/registry_sync_report.jtd.json:48` объявляет свойство с именем `ref`
> — это ключевое слово Rust, генератор эскейпит идентификатор в `ref_`, и
> `snake_case("ref_") = "ref_" != "ref"`. Значит избыточны **308** полевых
> rename, а не все 309; триста девятый — единственный носитель провода `"ref"`,
> и его снятие сдвинуло бы байты формата, у которого нет ни одного оракула.
> Поймал это сам пасс на посадке Ф4.2b-2 (потребитель ниже по течению), а не
> перечитывание находки: правило «снимается ТОЛЬКО тождественный rename»
> оказалось старше собственного числа. Свойство схемы, названное ключевым
> словом Rust, — постоянный класс, а не случайность этого дерева.

**Ключевой вопрос — станет ли `#[serde(rename)]` ненужным.** Автопроверка
всех полевых пар (awk-скрипт в §11): пар `идентификатор → wire`, где wire
НЕ равен snake_case(идентификатор), — **ноль**. Три образца:

1. *Однословное:* `pub path: String` + `#[serde(rename = "path")]`
   (`entry/mod.rs:17-18`). Идентификатор уже равен проводу; rename избыточен
   и сегодня, №2 его только не создаёт заново.
2. *Двусловное:* `pub contentHash: String` + `#[serde(rename = "content_hash")]`
   (`entry/mod.rs:185-186`). После переименования в `content_hash`
   идентификатор равен проводу ⇒ rename избыточен (serde по умолчанию берёт
   имя члена как есть).
3. *Wire не выводится из имени:* среди **полей** таких нет (проверка выше —
   ноль исключений) — **ЛОЖНО, см. поправку выше: ровно одно поле есть,
   `pub ref_` при проводе `"ref"`**; невыводимость живёт у **вариантов**:
   `KindName` → `"kind-name"` и `KindName0` → `"kind/name"`
   (`repomd/mod.rs:60-64`),
   `LazyPull` → `"lazy-pull"`. Их rename обязателен при любой форме.

**Общее правило (число ЛОЖНО — 308, не 309, см. поправку выше):**
№2 делает избыточными **все 309 полевых** `#[serde(rename)]`
(385 всего − 76 вариантных; вариантные 76 — 42 PackageKind + 9 DeliveryMode
+ 8 NamingConvention + 11 Event + 2 BindingSite + 2 OutcomeAction + 2
RepomdFileEntry — остаются: у части из них wire-строка невыводима ни из
какого case-правила). Удаление rename — не обязательная часть №2, но
«переименовал и оставил rename» — это 309 лишних строк, которые следующий
читатель примет за несводимость.

---

## 6. Радиус преобразования №3 (политика пустого) и №5 (умолчание скаляра)

**№3, площадка:** 31 сайт коллекций (таблица §3). Аннотированы сегодня 21,
значения распределены так: `emit` — 6 сайтов (by_name `packages`/`versions`,
repomd `files`, journal `renamed.from`/`renamed.to`, словниковый
`requires_any_entry.one_of`), `omit` — 15, все словниковые.
Рукописные двойники подтверждают обе политики: `NameEntry.packages:
Vec<PackageEntry>` без skip — пустое выходит как `[]`
(`crates/vibe-index/src/types/entry/aggregate.rs:86`, комментарий схемы
`by_name.jtd.json:19`), а `authors` со skip `Vec::is_empty` — опускается
(`crates/vibe-index/src/types/entry/mod.rs:79-80`).

**№5, разложение `Option<Box<T>>` (121 всего, U4) на три класса:**

| класс | полей | по файлам |
|---|---|---|
| T = коллекция (`Vec`/`HashMap`) | **45** | by_name 15, entry 15, journal 15 |
| T = скаляр (`String`/`bool`/`Version`) | **48** | by_name 15, journal 14, entry 13, list_report 5, install_plan 1 |
| T = структура | **28** | by_name 9, entry 9, journal 9, (+ Tombstone 1 в by_name) |

(Классы посчитаны шаблонами из §11; `Version` = алиас `String`, т.е. ref на
фрагмент-скаляр словника — скаляр по правилу пакета.)

**Полный список 48 скалярных полей и их `x-default`.** Схемный `x-default`
носят только **yanked** и **frozen** фрагмента `version_entry`
(`vocabularies.json:407-418`, оба `false`) — то есть 6 из 48 сайтов
(по 2 поля × 3 копии version_entry в by_name/entry/journal). Остальные
**42 сайта x-default не имеют**:

- by_name (15): `category`, `minVibeVersion`, `default`, `latestStable`
  (Version), `describes`×2, `description`×2, `supersededBy`, `frozen`✓,
  `homepage`, `license`, `resolvedCommit`, `yanked`✓, `commit`
  (`by_name/mod.rs:42,51,106,140,210,214,232,298,302,310,314,326,350,362,384`)
- entry (13): те же без `supersededBy`/`latestStable`
  (`entry/mod.rs:24,33,88,166,170,239,243,251,255,267,291,303,325`)
- journal (14): те же 13 + `version` плеча `Removed` (Version)
  (`journal/mod.rs:41,50,274,347,450,454,523,527,535,539,551,575,587,609`)
- list_report (5): `bootSnippet`, `overridden`, `registry`, `resolvedCommit`,
  `sourceRef` (`list_report/mod.rs:31,55,61,66,72`)
- install_plan (1): `bootSnippet` (`install_plan/mod.rs:46`)

**Во что преобразование обязано попасть — рукописные двойники:**

- С `x-default` — в скаляр с умолчанием и skip-предикатом:
  `crates/vibe-index/src/types/entry/mod.rs:135-137`:
  ```rust
  /// The version's withdrawal — a journal fact, not an authorial one: …
  #[serde(default, skip_serializing_if = "crate::types::is_false")]
  pub yanked: bool,
  ```
  (и так же `frozen`, строки 143-145). Это НЕ `Option<Box<bool>>` и НЕ голый
  `bool`: отсутствие на проводе и `false` — одно и то же.
- Без `x-default` — в `Option<T>` **без бокса**, с skip `Option::is_none`:
  `mod.rs:66-67`: `#[serde(default, skip_serializing_if = "Option::is_none")] pub resolved_commit: Option<String>,`;
  `aggregate.rs:29-30`: `pub latest_stable: Option<Version>,`;
  `content.rs:94-95`: `pub category: Option<String>,`;
  `record.rs:73` (плечо Removed): `version: Option<Version>,` — и, что важно,
  **без skip_serializing_if**: писатель журнала пишет `"version": null`
  (записано в самой схеме, `journal.jtd.json:125`).
- list_report-двойники — в vibe-cli (за пределами vibe-index), по форме те же
  `Option<String>`/`Option<bool>`.

**Следствие для конструкции №5 (измерено, не решено):** «умолчание» — не
всем 48 полям; у 42 двойник остаётся `Option`, и №5 для них — это снятие
`Box<…>`, а не скаляризация. Правило «нет x-default — ошибка генерации»
покраснеет на всех 42, если у правила нет третьего состояния («опционален
без умолчания»).

---

## 7. Радиус преобразования №8 (доменные типы по x-rust-type)

Все `pub type` хостового дерева — **15**:

| алиас | правая часть | файл:строка |
|---|---|---|
| Group | `String` | by_cap:32, by_name:95, by_purl:50, entry:77, journal:336 (×5) |
| Version | `String` | by_cap:58, by_name:236, by_purl:76, entry:177, journal:461 (×5) |
| Timestamp | `String` | by_name:218, entry:174, repomd:117, journal:458 (×4) |
| Entry | `VersionEntry` | entry:11 |

`x-rust-type` в хостовом доме — **8 аннотаций**: 5 в словнике
(`naming_convention`→`NamingConvention`, `group`→`Group`, `version`→`semver::Version`,
`timestamp`→`chrono::DateTime<Utc>`, `version_entry`→`VersionEntry`) + 2 в
журнале (корень→`JournalRecord`, `event`→`Event`) + 1 в by_purl
(`binding_site`→`BindingSite`).

**Аннотации, которые генератор НЕ отразил никак:**

1. **`version` → `semver::Version`**: схема
   `formats/vocabularies.json:31-37` («Semantic version string —
   `semver::Version` in code») vs выход `pub type Version = String;`
   (например `entry/mod.rs:177`). Имя алиаса совпало с именем фрагмента, а
   не с аннотацией; правая часть — String.
2. **`timestamp` → `chrono::DateTime<Utc>`**: `vocabularies.json:38-44` vs
   `pub type Timestamp = String;` (`entry/mod.rs:174`). Аннотация не отражена
   ни именем (имя — от фрагмента), ни типом.
3. **`group` → `Group`**: `vocabularies.json:24-30` («the `vibe_core::Group`
   newtype on the wire») vs `pub type Group = String;` (`entry/mod.rs:77`).
   Имя совпало (случайно: jtd-codegen называет алиас по ключу фрагмента),
   правая часть — String, не `vibe_core::Group`.
4. **корень журнала → `JournalRecord`**: `schemas/journal/e1/journal.jtd.json:4`
   (`"x-rust-type": "JournalRecord"`) vs выход `pub struct Journal`
   (`crates/vibe-wire/src/generated/journal/e1/journal/mod.rs:16`). Имя
   эмитировано от стема файла, аннотация проигнорирована; оракул даже
   импортирует его с переименованием — `use …journal::Journal as GeneratedJournal;`
   (`crates/vibe-index/tests/wire_parity_journal.rs:42`).

**Проверка поимённо: `x-rust-type` на определениях, вышедших структурой или
enum'ом (не алиасом), — совпадает ли имя аннотации с эмитированным?**

| аннотация | вышла как | совпадение |
|---|---|---|
| `version_entry` → `VersionEntry` | `pub struct VersionEntry` (entry:184, by_name, journal:468) | **да** |
| `event` → `Event` | `pub enum Event` (journal:89) | **да** |
| `naming_convention` → `NamingConvention` | `pub enum NamingConvention` (repomd:56, journal:361) | **да** (но вариант `kind/name` эмитирован артефактным именем `KindName0`, repomd:64 — сходимость с рукописным `NamingConvention` потребует переименования варианта) |
| `binding_site` → `BindingSite` | `pub enum BindingSite` (by_purl:40) | **да** |
| корень журнала → `JournalRecord` | `pub struct Journal` (journal:16) | **НЕТ** — единственное расхождение имени |

Радиус №8 по потребителям — ноль: ни `Group`/`Version`/`Timestamp`, ни
`VersionEntry` generated никто вне `generated/**` не называет (§8).

---

## 8. Что делают ПОТРЕБИТЕЛИ сгенерированных типов

**Импортёры `vibe_wire` — шесть файлов** (U2-команда):

1. `crates/vibe-cli/src/commands/init/helpers.rs:94` — **продуктовый**.
   ```rust
   use vibe_wire::generated::init_report::{
       InitReport, Outcome as WireOutcome, OutcomeAction,
   };
   ```
   Идентификаторы: типы `InitReport`, `Outcome` (как `WireOutcome`),
   `OutcomeAction::{Created,Kept}`; поля `ok, command, project, path,
   created, kept, outcomes` + `path, action, reason` вложенного — все
   однословные. Занимает строки 94-116 (≈23 строки). **Ни №2, ни №5, ни №8
   его не двигают: в init_report нет camelCase-полей, `Option<Box<>>` и
   алиасов.**
2-6. Пять оракулов `crates/vibe-index/tests/wire_parity_*.rs` — **тестовые**
   (dev-зависимость, U1):

| оракул | строк | импортирует из vibe_wire | как сравнивает |
|---|---|---|---|
| wire_parity_entry.rs | 162 | `Entry` (:28) | `serde_json::to_value(рукописный)` → `from_value` в сгенерированный → `to_value` → `assert_eq!(j1, j2)` (:144-161) — **по проводу** |
| wire_parity_by_name.rs | 201 | `ByName` (:30) | тот же kernel (:171-195) |
| wire_parity_inverted.rs | 100 | `ByCap`, `ByPurl` (:22-23) | kernel ×2 (:57-67, :78-95) |
| wire_parity_journal.rs | 395 | `Journal as GeneratedJournal` (:42) | kernel в цикле по всем 11 событиям (:332-388); «a union arm the schema forgot makes `from_value` itself fail» (:302) |
| wire_parity_repomd.rs | 130 | `Repomd as GeneratedRepomd` (:26) | kernel (:84-124) |

Цитата kernel (entry, :144-161): «`let j1 = serde_json::to_value(&handwritten)…
let generated: Entry = serde_json::from_value(j1.clone())… let j2 =
serde_json::to_value(&generated)… assert_eq!(j1, j2, "wire drift between
the hand-written and the generated entry…")». Сравнение — на
`serde_json::Value`, порядок ключей не важен (шапка файла, :14-16).

**Идентификаторы generated, которые называют оракулы:** только шесть имён
импорта — `Entry`, `ByName`, `ByCap`, `ByPurl`, `GeneratedJournal`,
`GeneratedRepomd`. **Ни одного идентификатора поля** сгенерированных типов
оракулы не называют: `grep -rnE "\.[a-z]+[A-Z]" crates/vibe-index/tests/wire_parity_*.rs`
→ пусто (exit 1) — конструкции строятся из рукописных типов vibe-index,
сгенерированный тип — opaque труба для Value. Это и есть причина, по которой
«оракулы переживают все преобразования» (WAL-факт `WAL-ORACLES-SURVIVE-F42`),
и она подтверждена замером.

**Форма зависимости (U1):** у `vibe-index` — **dev**-зависимость
(`Cargo.toml:55`, секция `[dev-dependencies]` с :44); у `vibe-cli` — обычная
(`Cargo.toml:31`, секция `[dependencies]` с :17).

**Радиус №2/№5/№8 по потребителям: ноль идентификаторов.** Единственный
продуктовый потребитель называет 3 типа init_report, которых семь
преобразований не меняют; оракулы называют 6 имён типов, которые меняет
только №8-сходимость имени `Journal` (см. §7 п.4 — переименование в
`JournalRecord` тронет 1 строку импорта оракула).

---

## 9. Семь вложенных структур — девятый случай, который ни одно преобразование не называет

Рукописный `VersionEntry` объявляет семь полей-СТРУКТУР со значением по
умолчанию и skip-предикатом `…::is_empty`
(`crates/vibe-index/src/types/entry/mod.rs`):

```rust
#[serde(default, skip_serializing_if = "CompatibilityEntry::is_empty")]
pub compatibility: CompatibilityEntry,        // :94-95
#[serde(default, skip_serializing_if = "ProvidesEntry::is_empty")]
pub provides: ProvidesEntry,                  // :97-98
#[serde(default, skip_serializing_if = "RequiresEntry::is_empty")]
pub requires: RequiresEntry,                  // :100-101
#[serde(default, skip_serializing_if = "ObsoletesEntry::is_empty")]
pub obsoletes: ObsoletesEntry,                // :106-107
#[serde(default, skip_serializing_if = "ConflictsEntry::is_empty")]
pub conflicts: ConflictsEntry,                // :109-110
#[serde(default, skip_serializing_if = "FeaturesEntry::is_empty")]
pub features: FeaturesEntry,                  // :112-113
#[serde(default, skip_serializing_if = "I18nEntry::is_empty")]
pub i18n: I18nEntry,                          // :118-119
```

Предикаты (цитаты):
- `CompatibilityEntry::is_empty` = `self.min_vibe_version.is_none() && self.requires_kinds.is_empty()` (`relations.rs:22-24`)
- `ProvidesEntry::is_empty` = `self.capabilities.is_empty()` (`relations.rs:34-36`)
- `RequiresEntry::is_empty` = `self.packages.is_empty() && self.capabilities.is_empty()` (`relations.rs:48-50`)
- `ObsoletesEntry::is_empty` = `self.packages.is_empty()` (`relations.rs:65-67`)
- `ConflictsEntry::is_empty` = `self.packages.is_empty()` (`relations.rs:77-79`)
- `FeaturesEntry::is_empty` = `self.features.is_empty() && self.exclusive.is_empty()` (`content.rs:44-46`)
- `I18nEntry::is_empty` = `self.available.is_empty() && self.default.is_none()` (`content.rs:78-80`)

Сгенерированное объявление тех же семи полей — `Option<Box<…Entry>>` со
skip `Option::is_none` (например `entry/mod.rs:229-235`):

```rust
#[serde(rename = "compatibility")]
#[serde(skip_serializing_if = "Option::is_none")]
pub compatibility: Option<Box<CompatibilityEntry>>,
```

**Что различается на проводе.** Рукописный писатель пустую структуру
**опускает** — предикат `is_empty` истинен на пустой, ключ не пишется;
`{"provides":{}}` рукописная сторона не пишет НИКОГДА. Сгенерированный тип
опускает ключ только при `None`, но `Some(пустая-структура)` сериализует как
`{}` — форму, которую рукописная сторона не порождает, но сгенерированный
читатель принимает и (что важнее) переизлучает. В схеме эти семь полей —
`optionalProperties` с `ref` на структуру (`vocabularies.json:359-395`),
без всякой аннотации: ни `x-empty` (они не коллекции), ни `x-default`.

**Вывод: это ДВЕ Rust-формы над ОДНОЙ грамматикой провода с неизвестной
семантикой пустого объекта.** Для «ключа нет» формы неразличимы; различие
живёт в состоянии `Some(empty)`/«присутствующий-пустой объект», которое
оракулы не упражняют (их фикстуры полностью заполнены —
`FULLY_POPULATED_KEY_COUNT = 33`, `wire_parity_entry.rs:35`), и которое при
будущей замене рукописных типов реэкспортом сгенерированных меняет
наблюдаемое поведение: value-struct с `is_empty`-skip не может выразить
«пустой, но присутствующий», а `Option<Box<>>` может. Ни одно из восьми
преобразований этот случай не называет: №3 — про коллекции, №5 — про
скаляры, №8 — про имена типов. Это девятая форма, и ей нужна запись в
схеме/правиле, прежде чем реэкспорт станет честным.

---

## 10. Дыры и неожиданности

1. **REGISTRY.toml ссылается на несуществующую схему.**
   `formats/REGISTRY.toml:195`: `schema = "schemas/hello/e1/hello.jtd.json"`,
   а `find schemas -type f` даёт 13 файлов — каталога `hello` нет. При этом
   `FormatId::Handshake` существует и объявлен `foreign_parsers = Many`
   (`format_id/mod.rs:93,119,145,171`). Преобразование №4 (реестр) и
   кодоген-сборка встретят этот формат раньше, чем кто-либо ожидал.
2. **«Нет аннотации — ошибка» краснеет на движке.** У схемы движка 4 enum'а,
   5 коллекций, 6 опциональных скаляров — все без аннотаций (U8), а крейт
   движка кампании запрещён. Правило обязано быть scoped на дом схемы, либо
   это отдельное решение владельца.
3. **42 скалярных сайта без `x-default`, чей двойник — честный `Option`.**
   Правило «нет аннотации — ошибка» без третьего состояния заставит
   аннотировать поля, у которых умолчания нет и быть не может
   (`resolved_commit`, `license`…). Либо у правила появляется явное
   «опционален без умолчания», либо аннотации обесцениваются.
4. **Пин генератора не сведён с артефактами — видно в дереве.** Шапки
   сгенерированных файлов печатают `jtd-codegen for Rust v0.2.1`
   (`entry/mod.rs:1`), а рукописный lib.rs:49 говорит «jtd-codegen 0.4.1».
   Постпроцессор привязан к форме эмиссии (своё же doc: «a changed shape
   means the pin moved», postproc.rs:20-23) — сводить до следующих
   преобразований, как уже записано в WAL (`WAL-KI-GENERATOR-PIN-UNRECONCILED`).
5. **Артефактное имя варианта `KindName0`** для wire `"kind/name"`
   (`repomd/mod.rs:63-64`) — коллизионный суффикс генератора. Сходимость с
   рукописным `NamingConvention` (у того варианты с осмысленными именами,
   `kinds.rs:88`) потребует переименования варианта — №8 по имени это не
   покрывает.
6. **`files_written` — два разных типа под одним wire-именем.** В
   install_report это `u32` (`install_report/mod.rs:26`), в list_report —
   `Vec<String>` (`list_report/mod.rs:37`, идентификатор `filesWritten`).
   №2 переименует оба; никакой коллизии, но читатель диффа должен знать,
   что это два разных поля.
7. **`oneOf` — camelCase в ОБЯЗАТЕЛЬНОМ поле.** `RequiresAnyEntry.oneOf:
   Vec<String>` (`entry/mod.rs:134-135`) — не `Option`, №5 его не касается,
   №2 — да. Двухсловных обязательных полей немного, но они есть
   (`schemaVersion`, `filesCount`, `packageCount`…).
8. **Словник — единственный дом словаря — вне периметра специмапа**
   (уже записано в WAL как `WAL-KI-VOCAB-OUTSIDE-THE-MAP`); замер это
   подтверждает косвенно: 18 фрагментов, из которых в `schemas/` не виден
   ни один.
9. **`[format.cli-package-tree]` живёт вне JTD-конвейера** — его схема
   `crates/vibe-cli/resources/package-tree.schema.v1.json` (REGISTRY.toml:93),
   JSON Schema, не JTD. Ни одно из семи преобразований его не трогает;
   отмечено, чтобы «13 схем» не прочитали как «13 JTD-форматов в реестре»
   (JTD-форматов в реестре 13 записей по 12 схемам: entry обслуживает и
   index-entry, и index-primary — REGISTRY.toml:104,120).

---

## 11. Как воспроизвести этот замер

Все команды — из корня worktree, Git Bash. Числа ВЕРДИКТА получены ими же.

```bash
# инвентарь домов схем (13 + словник + движок)
find schemas -type f | sort
ls formats/vocabularies.json formats/REGISTRY.toml
ls packages/org.vibevm.ai-native/core-ai-native/v0.8.0/schemas/

# оба сгенерированных дерева
find crates/vibe-wire/src/generated -type f | sort
find packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/generated -type f | sort

# U3: 81 camelCase-поле, разбивка по файлам
grep -rcE "pub [a-z]+[A-Z][A-Za-z0-9]*:" crates/vibe-wire/src/generated/ | grep -v ":0"
grep -rE "pub [a-z]+[A-Z][A-Za-z0-9]*:" crates/vibe-wire/src/generated/ | wc -l   # -> 81

# U4: Option<Box< по файлам (сумма 121; 40/37/38/5/1)
grep -rc "Option<Box<" crates/vibe-wire/src/generated/ | grep -v ":0"

# классы №5: коллекции 45 / скаляры 48 / структуры 28 (сумма 121)
grep -rhoE "Option<Box<(Vec<[^>]*>|HashMap<[^>]*, [^>]*>)>>" crates/vibe-wire/src/generated/ | wc -l
grep -rhoE "Option<Box<(String|bool|Version|Timestamp|Group|u32)>>" crates/vibe-wire/src/generated/ | wc -l
grep -rhoE "Option<Box<[A-Z][A-Za-z0-9]*>>" crates/vibe-wire/src/generated/ | grep -vE "<(String|bool|Version|u32|Timestamp|Group)>>" | wc -l

# №1: все pub enum + варианты
grep -rn "pub enum" crates/vibe-wire/src/generated/
for f in $(grep -rl "pub enum" crates/vibe-wire/src/generated/); do awk '
  /^pub enum /{name=$3;inf=1;count=0;next}
  inf&&/^}/{printf "%s %s = %d variants\n",FILENAME,name,count;inf=0;next}
  inf&&/^    [A-Z][A-Za-z0-9]*[ ,({]/&&!/^    #/{count++}' "$f"; done

# allow(non_snake_case) — единственный сайт
grep -rn "non_snake_case" crates/vibe-wire/src/ packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/generated/

# №2: ЭТОТ СКРИПТ ВАКУУМЕН — НЕ ЧИТАТЬ ЕГО ПУСТОЙ ВЫВОД КАК ДОКАЗАТЕЛЬСТВО
# (поправка 2026-08-17, §5). Шаблон ниже требует конца строки сразу после
# двоеточия, чего у эмитируемого поля не бывает: он молчит и на
# `pub ref_: String,` при проводе "ref", и на любом поле вообще. Пустой вывод
# доказывает не правило, а отсутствие измерения. Прежде чем читать пустой
# вывод как доказательство, скорми скрипту случай, который он ОБЯЗАН пометить.
# №2: доказательство «wire == snake_case(идентификатор)» для всех полей (пустой вывод = правило)
for f in $(find crates/vibe-wire/src/generated -name "*.rs"); do awk '
  /#\[serde\(rename = "/{if(match($0,/rename = "([^"]+)"/,m))wire=m[1];pend=1;next}
  pend&&/pub [A-Za-z]+[A-Za-z0-9]*: {0,1}$/{if(match($0,/pub ([A-Za-z0-9]+)/,id)){ident=id[1];pend=0;
    out="";for(i=1;i<=length(ident);i++){c=substr(ident,i,1);if(c~/[A-Z]/&&i>1)out=out"_";out=out tolower(c)}
    if(out!=wire)printf "%s ident=%s wire=%s snake=%s\n",FILENAME,ident,wire,out}}' "$f"; done

# serde(rename) всего/движок
grep -r "serde(rename" crates/vibe-wire/src/generated/ | wc -l            # 385
grep -r "serde(rename" packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/generated/ | wc -l  # 56
grep -rcE "pub [a-z]+[A-Z][A-Za-z0-9]*:" packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/generated/  # 12

# №4: deny_unknown_fields сегодня — ноль; структур — 69
grep -rc "deny_unknown_fields" crates/vibe-wire/src/generated/ | grep -v ":0"   # пусто
grep -r "pub struct" crates/vibe-wire/src/generated/ | wc -l                    # 69

# №6: Timestamp-поля (7) и алиасы (4)
grep -rc ": Timestamp" crates/vibe-wire/src/generated/ | grep -v ":0"
grep -rn "pub type" crates/vibe-wire/src/generated/

# №8: поля, типизированные Version/Group
grep -r ": Version\b" crates/vibe-wire/src/generated/ | wc -l   # 9
grep -r ": Group\b" crates/vibe-wire/src/generated/ | wc -l     # 13

# §3: аннотации по каждому файлу схем (строки «enum=… x-vocab=… coll=… x-empty=… x-default=… x-rust-type=…»)
for f in schemas/*.jtd.json schemas/index/e1/*.jtd.json schemas/journal/e1/*.jtd.json \
         formats/vocabularies.json \
         packages/org.vibevm.ai-native/core-ai-native/v0.8.0/schemas/specmap.jtd.json; do
  echo "$f enum=$(grep -c '\"enum\"' "$f") x-vocab=$(grep -c '\"x-vocabulary\"' "$f") \
coll=$(grep -cE '\"elements\"|\"values\"' "$f") x-empty=$(grep -c '\"x-empty\"' "$f") \
x-default=$(grep -c '\"x-default\"' "$f") x-rust-type=$(grep -c '\"x-rust-type\"' "$f")"; done
# (для словника coll=18 сырых вхождений = 16 сайтов: features/exclusive несут и values, и вложенный elements)

# U5/U6/U8
grep -rn "x-default" schemas/ formats/
grep -rn "\"type\": \"timestamp\"" schemas/ formats/ packages/org.vibevm.ai-native/core-ai-native/v0.8.0/schemas/
grep -cn "x-" packages/org.vibevm.ai-native/core-ai-native/v0.8.0/schemas/specmap.jtd.json   # 0

# U7 + дыра handshake
grep -n "foreign_parsers\|^schema\|^\[" formats/REGISTRY.toml
ls schemas/hello/e1/   # No such file or directory

# U1/U2
grep -n "vibe-wire" crates/vibe-cli/Cargo.toml crates/vibe-index/Cargo.toml
sed -n '15,31p' crates/vibe-cli/Cargo.toml; sed -n '44,56p' crates/vibe-index/Cargo.toml
grep -rn "vibe_wire" crates/ xtask/ --include="*.rs" | grep -v "crates/vibe-wire/"

# §8: оракулы
wc -l crates/vibe-index/tests/wire_parity_*.rs
grep -n "^use " crates/vibe-index/tests/wire_parity_*.rs
grep -rnE "\.[a-z]+[A-Z]" crates/vibe-index/tests/wire_parity_*.rs   # пусто: ни одного camelCase-доступа
```

Опечатки не должно быть, но числа главнее текста: если строка здесь и
строка в §1 разошлись — верна команда.
