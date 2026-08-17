# F42C3 — дублирование сгенерированных типов между схемами

Замер, не правка. Схемы проекта написаны на JTD, у JTD нет межфайловых
ссылок, поэтому общие фрагменты живут один раз в
`formats/vocabularies.json`, а слой кодогенерации подставляет их в каждую
схему, которая их называет (`metadata.x-vocabularies`), транзитивно.
Подстановка происходит на входе — значит на выходе один и тот же тип
эмитится столько раз, сколько схем его втянули, каждый раз в свой модуль
Rust. Этот замер превращает наблюдение в точный инвентарь и измеряет цену
трёх выходов. Никаких рекомендаций — выбор за владельцем.

**Периметр** (считано по нему): `crates/vibe-wire/src/generated/**`,
`formats/vocabularies.json`, `schemas/**`, `xtask/src/codegen/**`,
`crates/vibe-index/src/**` + `crates/vibe-index/tests/wire_parity_*.rs`,
`crates/vibe-cli/src/**`.

**Ключевые числа (итог, подробности ниже):**

| # | Метрика | Значение |
|---|---|---|
| 1 | Объявлений типов во всём сгенерированном дереве | **102** (`pub struct` / `pub enum` / `pub type`) |
| 2 | Уникальных имён | **58** |
| 3 | **Цена дублирования в штуках** | **44** лишних копии (102 − 58) |
| 4 | Имён, объявленных более чем в одном модуле | **18** — ровно числу фрагментов словника (совпадение не случайно, см. §3) |
| 5 | Из них номинально раздельных (struct/enum) | **15 имён = 48 копий**; остальные 3 имени (`Group`, `Version`, `Timestamp`) — 14 копий алиаса на внешний крейт, тип один |
| 6 | Побайтовых расхождений между копиями | **0 в коде, 0 в док-комментариях** (все 18 имён; инструмент проконтролирован красным, см. §2 и «Как воспроизвести») |
| 7 | Строк в дереве `generated/` | **2310** (19 `mod.rs`); на дублируемые объявления — **876** строк (только decl) / **1198** (decl + impl-блоки) = 37,9 % / 51,9 % |
| 8 | Строк, устранимых дедупликацией | **859** ((копии−1) × блок, decl + impl) |
| 9 | `pub use` в сгенерированном дереве сегодня | **0 ни одной строки** (grep exit 1, с контролем чувствительности) |
| 10 | Сверка «модули, где объявлен тип» ↔ «схемы, втянувшие фрагмент» | **18/18 — совпадает и счёт, и состав**; несовпадений нет |

Расхождение с иллюстрацией из пакета (§0.9): пакет говорил «PackageKind —
в пяти [модулях]»; дерево говорит **в семи** (`by_cap`, `by_name`,
`by_purl`, `entry`, `journal`, `list_report`, `registry_sync_report`).
`VersionEntry` в трёх и `NamingConvention` в двух — совпало. Правило
исполнено по дереву.

---

## 1. Инвентарь: какой тип в каких модулях (задача 3.1)

Периметр: все 19 `mod.rs` под `crates/vibe-wire/src/generated/`. Из них
5 — контейнерные (только `pub mod`, суммарно 48 строк), 14 несут типы:
13 схемных (по одному на `*.jtd.json`) плюс `format_id`, который эмитится
не из схемы, а из `formats/REGISTRY.toml` (`xtask/src/codegen/mod.rs:271`).

Обозначения: форма — `struct` / `enum` / `type`. Пути модулей сокращены
от `crates/vibe-wire/src/generated/`.

### 1.1. Типы более чем в одном модуле — 18 имён, 62 объявления

| тип | форма | модулей | модули |
|---|---|---|---|
| `PackageKind` | enum | **7** | by_cap, by_name, by_purl, entry, journal, list_report, registry_sync_report |
| `Group` | type | 5 | by_cap, by_name, by_purl, entry, journal |
| `Version` | type | 5 | by_cap, by_name, by_purl, entry, journal |
| `Timestamp` | type | 4 | by_name, entry, repomd, journal |
| `BootSnippetEntry` | struct | 3 | by_name, entry, journal |
| `CompatibilityEntry` | struct | 3 | by_name, entry, journal |
| `ConflictsEntry` | struct | 3 | by_name, entry, journal |
| `DeliveryMode` | enum | 3 | by_name, entry, journal |
| `FeaturesEntry` | struct | 3 | by_name, entry, journal |
| `I18nEntry` | struct | 3 | by_name, entry, journal |
| `ObsoletesEntry` | struct | 3 | by_name, entry, journal |
| `ProvidesEntry` | struct | 3 | by_name, entry, journal |
| `RequiresAnyEntry` | struct | 3 | by_name, entry, journal |
| `RequiresEntry` | struct | 3 | by_name, entry, journal |
| `SubskillEntry` | struct | 3 | by_name, entry, journal |
| `VersionEntry` | struct | 3 | by_name, entry, journal |
| `WorkspaceOriginEntry` | struct | 3 | by_name, entry, journal |
| `NamingConvention` | enum | 2 | repomd, journal |

Полные пути: `index/e1/{by_cap,by_name,by_purl,entry,repomd}`,
`journal/e1/journal`, `list_report`, `registry_sync_report`.

### 1.2. Типы ровно в одном модуле — 40 имён, 40 объявлений

| модуль | типы (форма) |
|---|---|
| `format_id` | `FormatId` (enum), `ForeignParsers` (enum) |
| `index/e1/by_cap` | `ByCap` (struct) |
| `index/e1/by_name` | `ByName` (struct), `PackageEntry` (struct), `Tombstone` (struct) |
| `index/e1/by_purl` | `ByPurl` (struct), `BindingSite` (enum) |
| `index/e1/entry` | `Entry` (type; алиас на `VersionEntry` внутри того же модуля, `entry/mod.rs:11`) |
| `index/e1/repomd` | `Repomd`, `RepomdFileEntryDirectory`, `RepomdFileEntryFile` (struct), `RepomdFileEntry` (enum) |
| `init_report` | `InitReport`, `Outcome` (struct), `OutcomeAction` (enum) |
| `install_plan` | `InstallPlan`, `PlanEntry` (struct) |
| `install_report` | `InstallReport`, `AppliedReport` (struct) |
| `journal/e1/journal` | `JournalRecord` (struct), `Event` (enum), `EventChannelSet`, `EventChannelUnset`, `EventEntrySetReplaced`, `EventForceReplaced`, `EventFrozen`, `EventInitialised`, `EventNotice`, `EventPublished`, `EventRemoved`, `EventRenamed`, `EventYanked` (11 struct) |
| `list_report` | `ListReport`, `ListEntry` (struct) |
| `registry_publish_report` | `RegistryPublishReport` (struct) |
| `registry_sync_report` | `RegistrySyncReport`, `RefreshedEntry`, `SkippedEntry` (struct) |
| `uninstall_report` | `UninstallReport` (struct) |

**Сверка суммы:** 62 (мульти) + 40 (одиночные) = **102 объявления** — равно
общему числу объявлений по grep (см. «Как воспроизвести», команда A2: 102).
Имена: 18 + 40 = **58 уникальных**.

Аналитическое замечание к строке 5 таблицы ключевых чисел: дублирование
`Group`/`Version`/`Timestamp` номинально безвредно — каждая копия это
`pub type Group = vibe_core::Group;` и подобные, т. е. все пять `Group`
суть один и тот же внешний тип. Настоящая номинальная боль — 48 копий
15 struct/enum имён: `a::VersionEntry` и `b::VersionEntry` суть разные
типы, значение одного нельзя положить туда, где ждут другой.

---

## 2. Идентичны ли копии побайтово (задача 3.2)

**Метод.** Экстрактор (python, весь скрипт в «Как воспроизвести», команда
B1) вырезает из каждого `mod.rs` каждое объявление целиком — ведущие
док-комментарии (`///`), атрибуты (`#[derive…]`, `#[serde…]`), заголовок и
тело до закрывающей `}` на нулевой колонке (для `pub type` — до `;`) — и
складывает каждое в свой файл в системном временном каталоге
(`<temp>/f42c3_decl/<ИмяТипа>/<модуль>.rs`, вне дерева репозитория).
Сравнение — построчное равенство всего блока («полное») и блока без
док-комментариев («только код»), расхождения печатаются unified diff.
Impl-блоки (`impl Serialize/Deserialize for …`) сравниваются отдельно,
**ключом по заголовку impl-блока** — см. ниже, почему это важно.

**Результат.** Все 18 мульти-модульных имён: **полное совпадение копий
побайтово, включая док-комментарии**. Расхождений в коде — 0; расхождений
в доке — 0. Ручных impl-блоков у дублируемых типов два семейства, и там
тоже построчное совпадение при сравнении подобное с подобным:

| семейство impl | копий | вердикт |
|---|---|---|
| `impl Serialize for PackageKind` | 7 | идентичны (by_cap:46, by_name:173, by_purl:63, entry:138, journal:398, list_report:84, registry_sync_report:43) |
| `impl<'de> Deserialize<'de> for PackageKind` | 7 | идентичны (…:64/191/81/156/416/102/61) |
| `impl Serialize for DeliveryMode` | 3 | идентичны (by_name:72, entry:58, journal:72) |
| `impl<'de> Deserialize<'de> for DeliveryMode` | 3 | идентичны (by_name:87, entry:73, journal:87) |

**Диффов нет — потому что расхождений нет**, и это не молчание
инструмента: красный контроль в «Как воспроизвести» (команда B3) гоняет
тот же экстрактор+diff по двум заведомо разным объявлениям (`Repomd` и
`ByName`) и получает различие (diff exit 1), а по заведомо одинаковым
копиям `VersionEntry` — exit 0.

**Почему доки не расходятся.** Гипотеза пакета была, что док приходит из
описания того места схемы, где фрагмент подставлен, и потому может
отличаться. Измерение её опровергает: док каждой копии — это дословно
`metadata.description` самого фрагмента словника. Пример: док всех трёх
`VersionEntry` (`entry/mod.rs:225`, `by_name/mod.rs:275`,
`journal/mod.rs:487`) побайтово равен `description` фрагмента
`version_entry` (`formats/vocabularies.json:291`). Документация
однократно сточена в словнике и размножается вместе с фрагментом —
расхождения дока могли бы возникнуть только при правке словника «на месте»,
чего слой не делает (копия не переписывается, `vocabulary.rs:96`).

**Методическая честность.** Первая версия компаратора impl-блоков сравнивала
их плоским списком и сообщила «impls_identical=False» для `PackageKind` и
`DeliveryMode` — ложное срабатывание: она сравнивала `impl Serialize` с
`impl Deserialize` одного модуля. Инструмент, выдающий различие на
одинаковом, так же недостоверен, как выдающий одинаковое на различном;
компаратор переведён на ключ заголовка, после чего семейства сошлись
полностью. Урок зафиксирован здесь, потому что «инструмент паниковал на
правильном ответе» — та же болезнь, что «молчал на неправильном».

**Док и код разделены явно:** расхождений в док-комментариях — 0,
расхождений в коде (поля, порядок, derive, атрибуты, impl-блоки) — 0.
Классы расхождений пусты оба, независимо друг от друга.

---

## 3. Граф: какая схема какие фрагменты тянет (задача 3.3)

Прочитаны `formats/vocabularies.json` (18 фрагментов верхнего уровня) и
все 13 схем под `schemas/`. Прямые тяги — ключ `metadata.x-vocabularies`
схемы; транзитивность — тот же ключ у самого фрагмента (есть у
`compatibility_entry` → `package_kind`, `subskill_entry` → `delivery_mode`,
`version_entry` → 15 фрагментов). Замыкание посчитано обходом (скрипт —
команда C1 в «Как воспроизвести»).

### 3.1. Схема → прямые фрагменты → транзитивное замыкание

| схема | прямые | замыкание (шт.) |
|---|---|---|
| `index/e1/by_cap` | group, package_kind, version | 3 |
| `index/e1/by_purl` | group, package_kind, version | 3 |
| `index/e1/repomd` | naming_convention, timestamp | 2 |
| `index/e1/by_name` | group, timestamp, version, version_entry | **17** |
| `index/e1/entry` | version_entry | **17** |
| `journal/e1/journal` | group, naming_convention, timestamp, version, version_entry | **18** (весь словник) |
| `list_report` | package_kind | 1 |
| `registry_sync_report` | package_kind | 1 |
| `init_report`, `install_plan`, `install_report`, `registry_publish_report`, `uninstall_report` | — | 0 |

Замыкание 17 у `by_name`/`entry` — это весь словник минус
`naming_convention`; у `journal` — все 18.

### 3.2. Фрагмент → сколько схем его втянуло (транзитивно)

`package_kind` **7** · `group` 5 · `version` 5 · `timestamp` 4 ·
`boot_snippet_entry`, `compatibility_entry`, `conflicts_entry`,
`delivery_mode`, `features_entry`, `i18n_entry`, `obsoletes_entry`,
`provides_entry`, `requires_any_entry`, `requires_entry`, `subskill_entry`,
`version_entry`, `workspace_origin_entry` — все по **3** ·
`naming_convention` **2**. Фрагментов, которых не тянет ни одна схема, — 0.

### 3.3. Сверка с инвентарём §1

Для каждого из 18 дублируемых имён: **число модулей, где тип объявлен =
числу схем, втянувших фрагмент, и составы совпадают имя-в-имя** (схема
`index/e1/by_name` ↔ модуль `generated/index/e1/by_name` и т. д.).
`package_kind`→`PackageKind`: 7 = 7; `group`→`Group`: 5 = 5; …
`naming_convention`→`NamingConvention`: 2 = 2 (repomd, journal — с обеих
сторон). **Несовпадений нет — 18 из 18.** Ни одного типа, дублируемого
вопреки графу, и ни одного фрагмента, дублирование которого не объяснилось
бы графом, замер не нашёл.

---

## 4. Где живёт подстановка в коде (задача 3.4)

Модуль — `xtask/src/codegen/vocabulary.rs`; драйвер —
`xtask/src/codegen/mod.rs`.

Механика, коротко. Один раз на прогон `Vocabularies::load` читает
`formats/vocabularies.json` в отображение «имя → JTD-фрагмент»
(`vocabulary.rs:65`, вызов из `mod.rs:127`). Затем на каждую схему
`Vocabularies::resolve(schema)` (`vocabulary.rs:102`, вызов из
`mod.rs:234` внутри цикла `generate_into`, `mod.rs:227`): читает схему,
берёт `metadata.x-vocabularies`, считает транзитивное замыкание
(`closure`/`walk`, `vocabulary.rs:204`/`218`, с отказами на цикл, обрыв
цепи и висячий `ref`), вставляет каждый фрагмент замыкания в
`definitions` **scratch-копии** схемы — с удалённым у фрагмента ключом
`x-vocabularies` (`fragment_for_definitions`, `vocabulary.rs:341`) — и
пишет копию в нумерованный каталог временного каталога прогона
(`vocabulary.rs:179`). Возвращает путь к копии; схема на диске не
переписывается. Дальше драйвер порождает `jtd-codegen --rust-out <sub_out>
<resolved>` (`mod.rs:236`), вывод прогоняется через постпроцессинговые
проходы `rewrite_generated` (`mod.rs:266`; порядок проходов задокументирован
там же: boxing → snake_case → ordered maps → empty policy → optional
shapes → strictness → открытие словарей, пишущее те самые ручные impl).
Модуль каждого уровня дерева синтезируется отдельно (`mod.rs:176`).

**Видит ли слой глобальную картину — нет.** Состояние `Vocabularies` —
четыре поля: путь дома, разобранные фрагменты, scratch-каталог и счётчик
`issued` (`vocabulary.rs:46`). Замыкание вычисляется внутри `resolve` для
одной схемы и выбрасывается вместе с возвращённым путём; ничего не
накапливает ни «какой фрагмент попал в какую схему», ни тем более «этот
фрагмент уже эмилился в другой модуль». Эмиссия — отдельный подпроцесс на
каждую схему; один модуль генерации никогда не видит вывод другого.
Построить общий модуль из сегодняшнего слоя нельзя — но знание для него
добывается тем же самым замыканием, которое `resolve` уже считает и
выбрасывает: накопить `фрагмент → {схемы}` по одному `BTreeMap` на прогон
— механическое расширение, а не новая археология.

---

## 5. Три выхода — и цена каждого (задача 3.5)

Выбор не сделан — измерены цены.

### Выход №1 — общий модуль (`generated/shared/`), схемы получают реэкспорт

**Что делает.** Слой эмитит каждый словарный фрагмент один раз, в
собственный модуль; модули схем вместо копии определения несут
`pub use crate::generated::shared::VersionEntry;` и подобные.

**Чем измерена цена.** Переезжает **18 типов** (48 номинальных копий
struct/enum + 14 алиасов). `pub use` вместо объявления получают **8
модулей** (by_cap, by_name, by_purl, entry, repomd, journal, list_report,
registry_sync_report) — **62 строки реэкспорта** вместо 62 объявлений.
Из дерева уходит **859 строк** дублированных блоков (decl + impl,
(копии−1)); канонические копии — **339 строк** — переезжают в новый модуль.
Чистое сокращение ≈ **−797 строк** (34,5 % дерева; метод — экстрактор,
команда B2).

**Что ломает.** (а) Ни одной строки `pub use` сгенерированное дерево сегодня
не содержит (ключевое число 9) — это новый вид строки для постпроцессинговых
проходов: `rewrite_generated` переписывает файл после генерации, и каждый
проход должен либо узнавать реэкспорт и не трогать его, либо порядок
придётся пересматривать. (б) Драйвер генерирует по одной схеме за раз и не
носит глобального состояния (§4) — общий модуль меняет саму форму прогона:
сначала накопить карту фрагмент→схемы, потом эмитить общий, потом схемы.
(в) `check-codegen` — побайтовое сравнение перегенерации — сработает, но
первый же прогон перепишет 8 модулей разом.

**Что оставляет нерешённым.** (а) **Имя.** Каталог схем открыт; `shared`
не конфликтует ни с одной сегодняшней схемой, но конфликт имён — вопрос
политики, а не наличия. (б) **Эпохи.** Фрагмент не знает эпох, а схемы
живут в `index/e1` и `journal/e1`. Сегодня все потребители `version_entry`
— одной эпохи, но будущая e2-схема, потянувшая `version_entry`, заставит
выбирать: один общий модуль на все эпохи (смешивает эпохи) или
`shared/e1` (и дубли возвращаются при сосуществовании e1+e2). (в) **Доки.**
Сегодня расхождений нет (§2) — вопрос «куда девать разошедшиеся доки»
пуст, и выход №1 замораживает его таким навсегда: док один, из словника.
(г) Корневые типы схем (`ByName`, `JournalRecord`, …) дубликатами не
являются и в общий модуль не переезжают — доля ручного связывания между
схемами остаётся.

### Выход №2 — один модуль выигрывает, прочие копии без потребителя

**Что делает.** Потребитель (`vibe-index`) реэкспортирует словарные типы из
одного модуля (естественный кандидат — `entry`: его собственный док
называет его «корнем, который именует записи», `entry/mod.rs:6`), копии в
остальных модулях остаются в дереве без потребителя.

**Чем измерена цена.** При «entry выигрывает» без потребителя остаются
**45 из 62 дублированных объявлений** (by_name 17, journal 18, by_cap 3,
by_purl 3, repomd 2, list_report 1, registry_sync_report 1). Корневые типы
не дублируются вовсе, поэтому «один модуль» не может обслужить весь
контракт: `ByCap`, `ByPurl`+`BindingSite`, `Repomd`+3, `JournalRecord`+
`Event`+11 `Event*`, `ByName`+`PackageEntry`+`Tombstone` не имеют другого
дома — потребитель в любом случае тянет несколько модулей. Сегодняшнее
положение дел и того жёстче: у сгенерированного дерева **нет рантайм-потребителя,
кроме `vibe-cli`** (единственный use — `vibe_wire::generated::init_report`
в `crates/vibe-cli/src/commands/init/helpers.rs:94`); `vibe-index/src`
не использует generated вообще — его связи со сгенерированными типами
держат только parity-тесты (`crates/vibe-index/tests/wire_parity_*.rs`,
5 файлов), которые сверяют рукописные типы с generated на байтовую
эквивалентность провода.

**Что ломает — ссылками, не рассуждением.** `by_name::ByName.packages` —
`Vec<PackageEntry>` (`by_name/mod.rs:21`); `by_name::PackageEntry.versions`
— `Vec<VersionEntry>` (`by_name/mod.rs:148`); этот `VersionEntry` объявлен
в том же модуле (`by_name/mod.rs:277`) и номинально отличен от
`entry::VersionEntry` (`entry/mod.rs:229`; алиас `entry::Entry` —
`entry/mod.rs:11` — указывает на копию entry). Rust не приводит
`Vec<entry::VersionEntry>` к `Vec<by_name::VersionEntry>` ни коаерсией,
ни `PartialEq` между ними — потребитель, взявший `entry::VersionEntry`
как «единственный» `VersionEntry`, **не может пользоваться
`by_name::ByName` иначе как через преобразование поле-за-поле** (или
serde-транзит). То же для каждой пары модулей. Выход №2 вынуждает либо
растить конвертеры там, где типы одинаковы побайтово (§2), либо признать,
что «неиспользуемые» копии на деле используются — номинально, изнутри
своих корневых типов.

**Что оставляет нерешённым.** Само дублирование никуда не девается —
45 мёртвых копий остаются в дереве, `check-codegen` продолжает их
перегенерировать, а связь «корень тянет свою копию словаря» остаётся
неявной, выраженной только типами полей.

### Выход №3 — оставить как есть

**Что делает.** Ничего в кодогенерации; дублирование сохраняется.

**Чем измерена цена.** Рукописными остаются **23 типа-зеркала** в
`vibe-index` (grep по `crates/vibe-index/src/{types,journal,index}`):
`VersionEntry` (`types/entry/mod.rs:50`); `PackageEntry`, `Tombstone`,
`NameEntry` (`types/entry/aggregate.rs:25/62/80`);
`WorkspaceOriginEntry`, `FeaturesEntry`, `DeliveryMode`, `SubskillEntry`,
`I18nEntry`, `BootSnippetEntry` (`types/entry/content.rs:18/36/51/58/70/88`);
`CompatibilityEntry`, `ProvidesEntry`, `RequiresEntry`, `RequiresAnyEntry`,
`ObsoletesEntry`, `ConflictsEntry` (`types/entry/relations.rs:14/28/40/54/59/71`);
`PackageKind`, `NamingConvention` (`types/kinds.rs:21/88`); `Repomd`,
`RepomdFileEntry` (`types/repomd.rs:19/43`); `JournalRecord`, `Event`
(`journal/record.rs:22/34`); `BindingSite` (`index/inverted.rs:89`).
Из них **15 дублируют словарные имена** (все 15 номинально-дублируемых
struct/enum имён имеют рукописного двойника) и 8 — корневые типы схем.
Плюс **вторая рукописная `NamingConvention`** вне vibe-index —
`crates/vibe-core/src/manifest/project.rs:271`, ровно та, о которой
говорит `description` фрагмента `naming_convention`
(`formats/vocabularies.json:23`: «Two copies exist in code … Phase 4.2
converges the copies»). Алиасы `Group`/`Version`/`Timestamp` рукописных
двойников не имеют — обе стороны ссылаются на одни внешние типы.

**Что ломает.** Ничего — это статус-кво; именно поэтому его цена измеряется
не поломками, а остатком: 44 лишних копии в generated (ключевое число 3),
859 дублированных строк (число 8) и 23+1 рукописных зеркал, чью
эквивалентность проводу держат только parity-тесты.

**Что оставляет нерешённым.** Вся номинальная несовместимость копий
(§1, замечание о 48 копиях): всякое новое место, где хочется передать
запись из `journal` туда, где ждут `entry`, упирается в разные типы при
побайтово одинаковом содержимом.

---

## 6. Побочные вопросы, ответами-числами (задача 3.6)

**Сколько всего строк и сколько на дубликаты.** Всего в
`crates/vibe-wire/src/generated/` — **2310 строк** в 19 `mod.rs`
(5 контейнерных — 48 строк, 14 с типами — 2262; `wc -l`, команда A1).
Метод доли дубликатов: экстрактор §2 считает строки блоков каждого
объявления (док + атрибуты + тело) и impl-блоков. На дублируемые
объявления приходится **876 строк** самих объявлений (все копии) и
**1198 строк** вместе с impl-блоками — 37,9 % и 51,9 % дерева; устранимо
дедупликацией **859** ((копии−1) × блок; команда B2).

**Прецедент `pub use` в сгенерированном файле.** **Нет — ни одной строки.**
`grep -rn "pub use" crates/vibe-wire/src/generated/` → выход пустой, exit 1
(команда D1). Что это молчание инструмента, а не его глухота, показано
контролем: тот же шаблон на файле, где `pub use` заведомо есть
(`crates/vibe-index/src/lib.rs:33`), находит строку (exit 0, команда D2).
Слой сегодня не эмитит ни реэкспортов, ни чего-либо подобного; ближайший
существующий приём — внутри-модульный алиас (`pub type Entry =
VersionEntry;`, `entry/mod.rs:11`) и алиасы на внешние крейты, но
межмодульного реэкспорта не существует.

---

## Как воспроизвести

Все команды запускаются из корня worktree. Реальный вывод приведён
дословно; прогоны повторены перед сдачей.

### A. Инвентарь и общие числа

```console
$ wc -l crates/vibe-wire/src/generated/mod.rs crates/vibe-wire/src/generated/*/mod.rs \
    crates/vibe-wire/src/generated/*/*/mod.rs crates/vibe-wire/src/generated/*/*/*/mod.rs
  ... 2310 total                      # A1: строк в дереве (19 mod.rs; корневой mod.rs — отдельно, glob его не берёт)

$ grep -rho "^pub struct [A-Za-z0-9_]*\|^pub enum [A-Za-z0-9_]*\|^pub type [A-Za-z0-9_]*" \
    crates/vibe-wire/src/generated/ | wc -l
102                                   # A2: объявлений всего; exit 0

$ grep -rho "^pub struct [A-Za-z0-9_]*\|^pub enum [A-Za-z0-9_]*\|^pub type [A-Za-z0-9_]*" \
    crates/vibe-wire/src/generated/ | sed 's/^pub \(struct\|enum\|type\) //' | sort -u | wc -l
58                                    # A3: уникальных имён; exit 0; 102−58=44

$ find crates/vibe-wire/src/generated -name "mod.rs" | wc -l
19                                    # A4: модулей (5 контейнерных + 14 с типами)

$ python -c "import json;d=json.load(open('formats/vocabularies.json'));print(len(d.get('definitions',d)))"
18                                    # A5: фрагментов словника; exit 0
```

Таблица §1.1 одной строкой (счёт имён по модулям):

```console
$ grep -rnE "^pub (struct|enum|type) " crates/vibe-wire/src/generated/ \
  | sed -E 's#^crates/vibe-wire/src/generated/(.*)/mod\.rs:[0-9]+:pub (struct|enum|type) ([A-Za-z0-9_]+).*$#\3\t\2\t\1#' \
  | awk -F'\t' '{n[$1]++; if (m[$1] !~ $3) m[$1]=m[$1]","$3} END {for (k in n) printf "%d\t%s%s\n", n[k], k, m[k]}' | sort -rn
7       PackageKind,index/e1/by_cap,index/e1/by_name,index/e1/by_purl,index/e1/entry,journal/e1/journal,list_report,registry_sync_report
5       Version,index/e1/by_cap,index/e1/by_name,index/e1/by_purl,index/e1/entry,journal/e1/journal
5       Group,index/e1/by_cap,index/e1/by_name,index/e1/by_purl,index/e1/entry,journal/e1/journal
4       Timestamp,index/e1/by_name,index/e1/entry,index/e1/repomd,journal/e1/journal
3       WorkspaceOriginEntry,index/e1/by_name,index/e1/entry,journal/e1/journal
... (ещё 12 строк «3 …», затем) ...
2       NamingConvention,index/e1/repomd,journal/e1/journal
1       Tombstone,index/e1/by_name
...
```

### B. Идентичность копий (§2)

B1 — полный скрипт: экстракция (блок объявления = ведущие `///`/`#[`,
заголовок, тело до `}`/`;` на нулевой колонке; impl-блоки — отдельно),
побайтовое сравнение копий (с док-комментариями и без), счёт строк
дубликатов. Положить в системный temp (НЕ в дерево репозитория) и запустить
из корня worktree:

```python
import re, difflib, tempfile
from pathlib import Path

ROOT = Path("crates/vibe-wire/src/generated")
OUT = Path(tempfile.gettempdir()) / "f42c3_decl"; OUT.mkdir(exist_ok=True)
DECL = re.compile(r"^pub (struct|enum|type) ([A-Za-z0-9_]+)")

def blocks(p):                      # блок = ведущие /// и #[..], заголовок, тело
    lines = p.read_text(encoding="utf-8").splitlines(); out, i = [], 0
    while i < len(lines):
        m = DECL.match(lines[i])
        if m:
            form, name, start = m.group(1), m.group(2), i; j = i - 1
            while j >= 0 and (lines[j].startswith("///") or lines[j].startswith("#[")):
                start = j; j -= 1
            end = i
            if form == "type":
                while ";" not in lines[end]: end += 1
            else:
                while lines[end] != "}": end += 1
            out.append(("decl", name, start + 1, lines[start:end + 1])); i = end + 1; continue
        m2 = re.match(r"^impl.*\b([A-Za-z0-9_]+)\s*\{\s*$", lines[i]) if lines[i].startswith("impl") else None
        if m2:
            end = i
            while lines[end] != "}": end += 1
            out.append(("impl:" + lines[i], m2.group(1), i + 1, lines[i:end + 1])); i = end + 1; continue
        i += 1
    return out

names, impls = {}, {}               # имя -> [(модуль, строки)]; impl-ключ -> то же
for p in sorted(ROOT.rglob("mod.rs")):
    mod = p.relative_to(ROOT).parent.as_posix()
    for kind, name, a, text in blocks(p):
        (impls if kind.startswith("impl:") else names).setdefault(
            kind if kind.startswith("impl:") else name, []).append((mod, text))

total = sum(len(v) for v in names.values())
mods = sorted({m for occ in names.values() for m, _ in occ})
print(f"MODULES_WITH_DECLS={len(mods)}")
print(f"TOTAL_DECLS={total} UNIQUE_NAMES={len(names)}")
dup_decl_all = dup_blocks_all = dup_removal = 0
for name, occ in sorted(names.items()):
    d = OUT / name; d.mkdir(exist_ok=True)
    for mod, text in occ: (d / (mod.replace("/", "__") + ".rs")).write_text("\n".join(text) + "\n", encoding="utf-8")
    if len(occ) < 2: continue
    same = all(t == occ[0][1] for _, t in occ[1:])            # с док-комментариями
    code = [[l for l in t if not l.startswith("///")] for _, t in occ]
    same_code = all(c == code[0] for c in code[1:])            # только код
    print(f"{name}: copies={len(occ)} full_identical={same} code_identical={same_code}")
    if not same:
        for mod, t in occ[1:]:
            for l in difflib.unified_diff(occ[0][1], t, lineterm="", n=1): print("   " + l)
    first = occ[0][0]
    per_copy = len(occ[0][1]) + sum(len(t) for k, v in impls.items()
                                    for m, t in v if k.endswith(" " + name + " {") and m == first)
    dup_decl_all += len(occ) * len(occ[0][1]); dup_blocks_all += len(occ) * per_copy
    dup_removal += (len(occ) - 1) * per_copy
for key, occ in sorted(impls.items()):
    if len(occ) > 1:
        same = all(t == occ[0][1] for _, t in occ[1:])
        print(f"impl [{key}] copies={len(occ)} identical={same} at {','.join(m for m, _ in occ)}")
print(f"DUP_DECL_LINES_ALL_COPIES={dup_decl_all}")
print(f"DUP_BLOCKS_ALL_COPIES={dup_blocks_all}")
print(f"DUP_REMOVAL_LINES={dup_removal}")
```

B2 — его вывод (дословно, сокращено только многоточием в середине списка
имён; все 18 строк `full_identical=True code_identical=True`):

```console
$ python <путь-к-скрипту-выше>
MODULES_WITH_DECLS=14
TOTAL_DECLS=102 UNIQUE_NAMES=58
BootSnippetEntry: copies=3 full_identical=True code_identical=True
...
WorkspaceOriginEntry: copies=3 full_identical=True code_identical=True
impl [impl:impl Serialize for DeliveryMode {] copies=3 identical=True at index/e1/by_name,index/e1/entry,journal/e1/journal
impl [impl:impl Serialize for PackageKind {] copies=7 identical=True at index/e1/by_cap,index/e1/by_name,index/e1/by_purl,index/e1/entry,journal/e1/journal,list_report,registry_sync_report
impl [impl:impl<'de> Deserialize<'de> for DeliveryMode {] copies=3 identical=True at index/e1/by_name,index/e1/entry,journal/e1/journal
impl [impl:impl<'de> Deserialize<'de> for PackageKind {] copies=7 identical=True at index/e1/by_cap,index/e1/by_name,index/e1/by_purl,index/e1/entry,journal/e1/journal,list_report,registry_sync_report
DUP_DECL_LINES_ALL_COPIES=876
DUP_BLOCKS_ALL_COPIES=1198
DUP_REMOVAL_LINES=859
```

B3 — красное доказательство №1: тот же приём обязан различать заведомо
разное и подтверждать заведомо одинаковое:

```console
$ diff /tmp/f42c3_decl/Repomd/index__e1__repomd.rs /tmp/f42c3_decl/ByName/index__e1__by_name.rs; echo "exit=$?"
1,5c1,5
< /// `repomd.json` — the per-index catalog manifest, epoch 1 (PROP-005 §2.4),
< /// modelled after RPM's `repomd.xml`. Root type `Repomd`: nine required
...
exit=1                                  # различие поймано

$ diff /tmp/f42c3_decl/VersionEntry/index__e1__entry.rs /tmp/f42c3_decl/VersionEntry/index__e1__by_name.rs; echo "exit=$?"
exit=0                                  # одинаковые копии — пустой diff
```

### C. Граф схем и фрагментов (§3)

C1 — замыкание `x-vocabularies` (фрагменты тянут фрагменты тем же ключом):

```python
import json
from pathlib import Path
voc = json.load(open("formats/vocabularies.json", encoding="utf-8"))
direct = {n: b.get("metadata", {}).get("x-vocabularies", []) for n, b in voc.items()}
def closure(n):                      # только ЗАВИСИМОСТИ n, без неё самой
    seen, st = set(), list(direct[n])
    while st:
        f = st.pop()
        if f not in seen: seen.add(f); st.extend(direct.get(f, []))
    return seen
pull = {}
for p in sorted(Path("schemas").rglob("*.jtd.json")):
    d = json.load(open(p, encoding="utf-8"))
    frs = d.get("metadata", {}).get("x-vocabularies", [])
    full = set()                      # замыкание = сам фрагмент + его зависимости
    for f in frs:
        full.add(f); full |= closure(f)
    print(p, "direct:", sorted(frs), "closure:", len(full))
    for f in full: pull[f] = pull.get(f, 0) + 1
print("fragment -> schemas:", dict(sorted(pull.items(), key=lambda kv: -kv[1])))
```

Вывод (сокращён до решающих строк; пути — как их печатает `Path` на этой
платформе, с обратными слешами): `by_cap: direct=['group','package_kind',
'version'] closure=3`; `by_name: direct=['group','timestamp','version',
'version_entry'] closure=17`; `entry: direct=['version_entry']
closure=17`; `journal: direct=['group','naming_convention','timestamp',
'version','version_entry'] closure=18`; пять отчётных схем — `closure=0`.
Хвост: `fragment -> schemas: {'package_kind': 7, 'group': 5, 'version': 5,
'timestamp': 4, …тринадцать имён по 3…, 'naming_convention': 2}` —
в точности строки §1.1, сверка 18/18.

### D. Побочные вопросы (§6)

```console
$ grep -rn "pub use" crates/vibe-wire/src/generated/; echo "exit=$?"
exit=1                                  # D1: ни одного pub use в generated

$ grep -rn "^pub use" crates/vibe-index/src/lib.rs; echo "exit=$?"
33:pub use error::{Error, Result};
exit=0                                  # D2: контроль — тот же шаблон находит
```

Красное доказательство №2 — это пара D1/D2: пустой вывод D1 читается как
«таких нет» только потому, что D2 показывает — шаблон находит `pub use`,
когда он есть.

---

## Вердикт

Дублирование измерено и объяснено полностью: **102 объявления, 58 имён,
44 лишних копии**, все 18 дублируемых имён побайтово идентичны между
модулями — включая док-комментарии, потому что док размножается вместе с
фрагментом из единого словника. Число модулей у каждого типа в точности
равно числу схем, втянувших его фрагмент (18/18), — дублирование не
разрослось ни на строку сверх графа подстановки. Слой подстановки глобальной
картины не имеет, но вычисляет её по частям в каждом `resolve`. Цена
выходов: №1 — −797 строк и структурная перестройка прогона при пустом
сегодня вопросе доков; №2 — 45 мёртвых копий и номинальная
несовместимость `by_name::VersionEntry` с `entry::VersionEntry` при
побайтовом равенстве; №3 — 23+1 рукописных зеркала и 859 дублированных
строк как постоянный остаток. Выбор — за владельцем.
