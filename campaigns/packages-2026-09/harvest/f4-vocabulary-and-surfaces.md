# F4-VOCAB — словари, публикуемые поверхности и инвентарь под схемы Ф4.1

Замер статический, только чтение (git/cargo не использовались). Периметр
чтения: `crates/**`, `schemas/**`, `formats/**`, `xtask/**`; перепись словарей —
`crates/**` вне `**/generated/**`. Каждое утверждение несёт цитату `файл:строка`
(пути от корня рабочего дерева, строки — по текущему состоянию файлов).

## 1. ВЕРДИКТ

**Три прямых ответа:**

1. **Словарей, больных раздвоением (одно и то же имя типа определено ≥2 раз
   в периметре переписи), — 5, а не 1:** `BindingSite`,
   `PackageKind`, `NamingConvention`, `DeliveryMode`, `SourceKind`
   (разбор в §4). Из них правила сериализации РАСХОДЯТСЯ сегодня только у
   `BindingSite` (kebab vs lowercase, §4.2.1); остальные четыре пары несут
   одинаковые правила и одинаковые строки провода — расхождение у них
   латентное (видимое значение см. §4.2.2–4.2.5). Записан как известный
   дефект был один (`BindingSite`); ещё четыре — находка этого замера.
2. **Публикуемых поверхностей без ридера ФАЙЛА — 3, а не 2:**
   `by-cap/<slug>.jsonl`, `by-purl/<slug>.jsonl` и `primary.jsonl.gz`
   (запись U2 не называла третью). У `primary.jsonl`, `by-name/<name>.json`
   и `repomd.json` ридеры файла есть (§5).
3. **Вариантов у события журнала — 11** (посчитано командой, §6 и §9):
   Initialised, Published, Frozen, Yanked, Removed, Renamed, Notice,
   ChannelSet, ChannelUnset, ForceReplaced, EntrySetReplaced.

**Оговорки.** Перепись §3 считает только enum'ы, у которых ВСЕ варианты
единичные (словарь-строка) и которые несут `Serialize` и/или `Deserialize`
(derive или рукописный impl) — по критерию задания. Тегированные объединения
(`Event` журнала, `RepomdFileEntry`, PROP-043-журнал `progress_core::Event`)
в перепись не входят — первые два разобраны отдельно (§6, U4). Вне переписи,
но в теме дублирования остались: две `generated`-копии `PackageKind` в
`vibe-wire` (исключены периметром), два определения `RecipeId` без serde
(§8, п. 8) и третья копия словаря `BindingSite` строковыми литералами
(§8, п. 2).

## 2. Подтверждение-или-опровержение записанного (U1–U7)

| # | записанное утверждение | вердикт | цитата |
|---|---|---|---|
| U1 | `BindingSite` определён дважды: писатель с `rename_all = "kebab-case"`, клиент с `rename_all = "lowercase"`; значения однословные, паритет-теста нет | **ПОДТВЕРЖДЕНО** | писатель: `crates/vibe-index/src/index/inverted.rs:88-92` (`#[serde(rename_all = "kebab-case")]`, варианты `Package`, `Subskill`); клиент: `crates/vibe-registry/src/index_client/wire.rs:92-99` (`#[serde(rename_all = "lowercase")]`, те же варианты). Паритет-теста нет: тесты писателя строят enum напрямую (`crates/vibe-index/src/index/inverted.rs:431-433`), тесты клиента парсят свой собственный JSON (`crates/vibe-registry/src/index_client/wire.rs:190-191`); ни один тест не сравнивает две копии |
| U2 | у `by-cap/<slug>.jsonl` и `by-purl/<slug>.jsonl` нет ридера файла, тогда как у `primary.jsonl` и `by-name/<name>.json` ридер есть | **ПОДТВЕРЖДЕНО, с расширением** | ридера файла нет: в `crates/vibe-index/src/index/inverted.rs` только писатели `write_capability` (`:200`) / `write_purl` (`:215`) и счётчики файлов `entry_count_capability`/`entry_count_purl` (`:241-247`) — функции чтения строк нет; grep по дереву других читателей не находит. Ридеры есть: `primary.jsonl` — `read`/`parse` `crates/vibe-index/src/index/primary.rs:75,84`; `by-name` — `read`/`read_all` `crates/vibe-index/src/index/by_name.rs:57,72`. **Расширение:** третья поверхность без ридера файла — `primary.jsonl.gz` (§5) |
| U3 | `PackageKind` записан в дереве дважды — в `vibe-core` и в `vibe-index` — и держится паритет-тестом | **ОПРОВЕРГНУТО наполовину: дубль есть, паритет-теста НЕТ** | дубль: `crates/vibe-core/src/package_ref/kind.rs:30-31` и `crates/vibe-index/src/types/kinds.rs:19-21` (обе `#[serde(rename_all = "lowercase")]`, по 6 вариантов). Паритет-тест не существует: док-комментарий обещает его (`crates/vibe-index/src/types/kinds.rs:4-5` «parity-test it (slice 3)», `:27-28` «the parity test below keeps the copies honest»), но в тестах того же файла (`:148-167`) проверяется только копия `vibe-index`; `crates/vibe-index/tests/content_hash_parity.rs` (единственный файл со словом «parity» в тестах) сверяет хеши контента, а не словарь (`:104-138`); в `vibe-index` нет ни одного теста, сопоставляющего обе копии (grep `vibe_core::PackageKind` по `crates/vibe-index` даёт только конверсию `crates/vibe-index/src/scanner/manifest.rs:23,66-75`). Реальная защита — исчерпывающий `match`-конвертер (ломает компиляцию при новом варианте) + односторонние селф-тесты; rename на одной из сторон прошёл бы молча |
| U4 | объединение `RepomdFileEntry` тегировано симметрично (`tag = "kind"`), и есть тест, утверждающий ОТКАЗ на записи без тега | **ПОДТВЕРЖДЕНО** | `crates/vibe-index/src/types/repomd.rs:42` (`#[serde(tag = "kind", rename_all = "lowercase")]`); тест `untagged_file_shape_is_rejected_not_guessed` — `crates/vibe-index/src/types/repomd.rs:137-145` (`assert!(parsed.is_err())` на `{"size":…,"sha256":…}` без `kind`) |
| U5 | словарь событий журнала несёт одиннадцать вариантов | **ПОДТВЕРЖДЕНО** | `crates/vibe-index/src/journal/record.rs:34-110`; подсчёт командой из §9 даёт `11` |
| U6 | часть вариантов события проецируется, часть отказывается по имени с называемой причиной | **ПОДТВЕРЖДЕНО** | проецируются 6: `crates/vibe-index/src/journal/project.rs:85-134`; отказ по имени 5: `project.rs:135-149` (`Event::Renamed/Notice/ChannelSet/ChannelUnset/ForceReplaced` → `unprojectable(variant, carrier)`); причина в сообщении: `project.rs:205-211` («the journal holds a \`{variant}\` record, but its carrier ({carrier}) is not built in this vibe-index…») |
| U7 | `NamingConvention` — закрытый словарь: неизвестное значение даёт громкий отказ | **ПОДТВЕРЖДЕНО по построению; прямого теста на отказ нет** | обе копии — закрытые enum'ы с по-вариантными `rename` и без `other`-варианта: `crates/vibe-index/src/types/kinds.rs:87-112`, `crates/vibe-core/src/manifest/project.rs:270-293`; serde обязан отказать на неизвестной строке. Но тесты проверяют только четыре валидных значения (`crates/vibe-index/src/types/kinds.rs:170-187`); теста вида «bogus ⇒ Err» для `NamingConvention` нет ни в `vibe-index`, ни в `vibe-core` (для `AuthKind` такой есть: `crates/vibe-core/src/manifest/project/tests.rs:110` `auth_kind_rejects_unknown_value`; для `PackageKind` есть: `crates/vibe-index/src/types/kinds.rs:156-159`) |

**Крупно, как просило задание:** опровержение U3 — заявленный паритет-тест
`PackageKind` не существует в дереве; док-комментарий `kinds.rs:4-5,27`
обещает то, чего нет. И находка сверх записи: больных словарей не один, а
пять (§1, §4).

## 3. Перепись словарей провода

Метод: двумя командами из §9 получены (а) все 229 enum-деклараций в
`crates/**` вне `generated/**`, (б) 63 enum'а с serde-derive в окне 8 строк
над декларацией. Из 63 исключены: 3 ложных срабатывания окна (serde-импорт
или чужой derive рядом, самого serde-derive у enum'а нет:
`CopyDest` `crates/vibe-cli/src/commands/tree/tui/copy/settings.rs:33-34`,
`ModelError` `crates/vibe-cli/src/commands/vvm/model.rs:17-18`,
`Mode` `crates/vibe-cli/src/output.rs:11-12`), 2 enum'а без serde вовсе
(`RecipeId` ×2 — см. §8), и 8 enum'ов с полями/тегированием (не словари-
строки: `ParamValue` `crates/vibe-actions/src/params.rs:56-63`; два `Event`
— `crates/vibe-index/src/journal/record.rs:32-34`,
`crates/progress-core/src/journal.rs:17-19`; `RepomdFileEntry`
`crates/vibe-index/src/types/repomd.rs:41-43`; `PublishPosture`
`crates/vibe-core/src/manifest/package.rs:238-240`; `WhenCondition`
`crates/vibe-core/src/manifest/package/when.rs:34-36`;
`RequiresPackageEntryWire` `crates/vibe-core/src/manifest/package/wire.rs:29-31`;
`VersionFieldWire` `crates/vibe-core/src/manifest/package/wire.rs:66-68`).
**Итог: 50 словарей.** «Правило» — атрибут на enum; «строки» — значения как
они уходят в JSON (для serde это вывод из правила; двусмысленных случаев два,
отмечены в §8, п. 9). «Паритет-тест» — тест, сверяющий копии между собой
(не селф-тест одной копии).

| # | тип | где определён | правило имён | строки провода | определений | расходятся | паритет-тест |
|---|---|---|---|---|---|---|---|
| 1 | RescanClass | `crates/progress-core/src/baseline.rs:194` | kebab-case | new, changed, carried-forward, control-sample | 1 | — | — |
| 2 | FactKind | `crates/progress-core/src/doc.rs:55` | lowercase | para, lead, item, cell | 1 | — | — |
| 3 | BlockKind | `crates/progress-core/src/doc.rs:68` | lowercase | text, code, comment, marker-only, heading | 1 | — | — |
| 4 | Severity | `crates/progress-core/src/doc.rs:106` | lowercase | error, warning | 1 | — | — |
| 5 | IssueCode | `crates/progress-core/src/doc.rs:113` | kebab-case | vocabulary, malformed, missing-attr, stranded, mid-paragraph, duplicate-status, wrapper-mismatch, unmarked, missing-anchor | 1 | — | — |
| 6 | Stage | `crates/progress-core/src/model.rs:14` | lowercase | unknown, idea, spec, impl, test, doc, freeze | 1 | — | — |
| 7 | State | `crates/progress-core/src/model.rs:36` | lowercase | hold, plan, work, done, void | 1 | — | — |
| 8 | Action | `crates/progress-core/src/model.rs:57` | lowercase | continue, drift, rework, remove | 1 | — | — |
| 9 | Audience | `crates/progress-core/src/model.rs:67` | lowercase | user, author, dev | 1 | — | — |
| 10 | MarkerForm | `crates/progress-core/src/model.rs:76` | lowercase | point, wrapper, shorthand | 1 | — | — |
| 11 | Granularity | `crates/progress-core/src/model.rs:88` | lowercase | document, section, paragraph, item, cell, fragment | 1 | — | — |
| 12 | View | `crates/progress-core/src/report.rs:15` | lowercase | done, todo, qa, remove, doc | 1 | — | — |
| 13 | FoldLoss | `crates/progress-core/src/rollup.rs:70` | lowercase | state, action, actionstage, audience, ref, comment | 1 | — | — |
| 14 | GateStatus | `crates/progress-core/src/state.rs:22` | lowercase | green, red, stale, unknown | 1 | — | — |
| 15 | Capability | `crates/vibe-actions/src/action.rs:147` | нет (идентификаторы Rust) | Safe, Mutating, Dangerous | 1 | — | — |
| 16 | ParamType | `crates/vibe-actions/src/params.rs:24` | lowercase | string, int, bool | 1 | — | — |
| 17 | Action | `crates/vibe-cli/src/commands/init/helpers.rs:191` | lowercase | created, kept | 1 (+generated-зеркало `OutcomeAction`, вне периметра) | нет (правила совпадают) | нет |
| 18 | LoadType | `crates/vibe-cli/src/commands/tree/model.rs:197` | lowercase | static, dynamic, none | 1 | — | — |
| 19 | DeclaredLink | `crates/vibe-cli/src/commands/tree/model.rs:206` | kebab-case | static, dynamic, static-transitive, static-hard | 1 (+словарь `LinkType`, см. §8 п. 3) | нет | нет |
| 20 | LoadOrigin | `crates/vibe-cli/src/commands/tree/model.rs:216` | kebab-case | declared, suggested, default, static-transitive, when-forced, none | 1 | — | — |
| 21 | ConditionKind | `crates/vibe-cli/src/commands/tree/model.rs:228` | lowercase | os | 1 | — | — |
| 22 | IndexKind | `crates/vibe-cli/src/commands/tree/model.rs:235` | lowercase | static, dynamic | 1 | — | — |
| 23 | SourceKind | `crates/vibe-cli/src/commands/tree/model.rs:243` | lowercase | registry, git, override, path, embedded, local | **2** (см. §4.2.5) | нет (правила и строки совпадают) | нет (только исчерпывающий match-конвертер `crates/vibe-cli/src/commands/tree/build.rs:429-437`) |
| 24 | Carrier | `crates/vibe-cli/src/commands/tree/model.rs:255` | по-вариантные rename | @spec, #use, #embed, #source | 1 | — | — |
| 25 | Severity | `crates/vibe-cli/src/commands/tree/model.rs:274` | lowercase | info, warn, error | 1 (омонимы с №4 — разные словари) | — | — |
| 26 | Kind | `crates/vibe-cli/src/commands/vvm/model.rs:39` | lowercase | tag, branch, commit | 1 | — | — |
| 27 | Profile | `crates/vibe-cli/src/commands/vvm/model.rs:100` | lowercase | debug, release | 1 | — | — |
| 28 | Origin | `crates/vibe-cli/src/commands/vvm/model.rs:185` | lowercase | managed, external, binary | 1 | — | — |
| 29 | SourceKind | `crates/vibe-core/src/manifest/lockfile.rs:199` | lowercase | registry, git, override, path, embedded, local | **2** (см. §4.2.5) | нет | нет |
| 30 | PackageFormat | `crates/vibe-core/src/manifest/package.rs:176` | lowercase | simple, normal | 1 | — | — |
| 31 | Materialization | `crates/vibe-core/src/manifest/package.rs:311` | kebab-case (Serialize) + рукописный Deserialize `:335-…` | copy, hardlink, in-place (устаревшее `snapshot` — громкий отказ, `:340-343`) | 1 | — | — |
| 32 | LinkType | `crates/vibe-core/src/manifest/package.rs:418` | lowercase + 2 по-вариантных rename (`:436,443`) | static, dynamic, static-transitive, static-hard | 1 (+словарь `DeclaredLink`, §8 п. 3) | нет | нет |
| 33 | BootCategory | `crates/vibe-core/src/manifest/package.rs:473` | kebab-case | foundation, flow, stack, tool, app, user-override | 1 | — | — |
| 34 | AuthKind | `crates/vibe-core/src/manifest/project.rs:164` | по-вариантные rename | none, token-env, credential-helper, ssh | 1 | — | есть селф-тест отказа: `crates/vibe-core/src/manifest/project/tests.rs:110` |
| 35 | NamingConvention | `crates/vibe-core/src/manifest/project.rs:271` | по-вариантные rename | fqdn, kind-name, name, kind/name | **2** (см. §4.2.3) | нет | нет |
| 36 | RefPolicy | `crates/vibe-core/src/manifest/redirect.rs:133` | kebab-case | pass-through-tag, pinned | 1 | — | — |
| 37 | DeliveryMode | `crates/vibe-core/src/manifest/subskill.rs:120` | kebab-case | eager, lazy-push, lazy-pull | **2** (см. §4.2.4) | нет | нет (только match-конвертер `crates/vibe-index/src/scanner/manifest.rs:183-188`) |
| 38 | PackageKind | `crates/vibe-core/src/package_ref/kind.rs:31` | lowercase | flow, feat, stack, tool, mcp, lang | **2** (см. §4.2.2) | нет | **нет** (заявлен, не существует — U3) |
| 39 | SlotIntegrity | `crates/vibe-core/src/user_config.rs:157` | kebab-case | trust-presence, verify | 1 | — | — |
| 40 | Status | `crates/vibe-index/src/cli/outdated.rs:50` | kebab-case | up-to-date, update-available, unknown | 1 | — | — |
| 41 | BindingSite | `crates/vibe-index/src/index/inverted.rs:89` | **kebab-case** | package, subskill | **2** (см. §4.2.1) | **ДА — правило; строки совпадают по случайности** | нет |
| 42 | DeliveryMode | `crates/vibe-index/src/types/entry/content.rs:51` | kebab-case | eager, lazy-push, lazy-pull | **2** (см. §4.2.4) | нет | нет |
| 43 | PackageKind | `crates/vibe-index/src/types/kinds.rs:21` | lowercase | flow, feat, stack, tool, mcp, lang | **2** (см. §4.2.2) | нет | нет |
| 44 | NamingConvention | `crates/vibe-index/src/types/kinds.rs:88` | по-вариантные rename (`:94,99,104,109`) | fqdn, kind-name, name, kind/name | **2** (см. §4.2.3) | нет | нет (только замороженные строки в селф-тесте `kinds.rs:170-187`) |
| 45 | BindingSite | `crates/vibe-registry/src/index_client/wire.rs:94` | **lowercase** | package, subskill | **2** (см. §4.2.1) | **ДА** | нет |
| 46 | WalkAttemptStatus | `crates/vibe-registry/src/multi_registry_resolver/attempt.rs:26` | kebab-case | not-found, public401 (см. §8 п. 9) | 1 | — | — |
| 47 | HitSource | `crates/vibe-trace/src/search.rs:43` | snake_case | spec, code | 1 | — | — |
| 48 | Verb | `crates/vibe-trace/src/select/parse.rs:40` | по-вариантные rename | implements, verifies, documents, deviates, informs | 1 | — | — |
| 49 | ToolChannel | `crates/vibe-workspace/src/tools.rs:27` | lowercase | binary, mcp | 1 | — | — |
| 50 | Affinity | `crates/vibe-mcp/src/agentic.rs:62` | нет (идентификаторы Rust) | AgenticOnly, StandaloneOnly, Both | 1 | — | — |

Омонимы `Severity` (№4 и №25), `Action` (№8 и №17), `State`/`View` и т.п. —
разные словари разных контекстов, совпадение имён типов не делает их одним
словарём (строки провода различаются или контексты не пересекаются).

## 4. Словари с несколькими определениями

Всего **5** (перечень в §1). Общее у всех пяти: после Ф4.2 «одним
определением» должен стать экземпляр в библиотеке-источнике истины формата
(для поверхностей каталога — `vibe-index::types`, потому что именно он пишет
провод), а второй стороне нужен реэкспорт: **сегодня `vibe-registry` вообще
не зависит от `vibe-index` как от crate** (grep `vibe_index` по
`crates/vibe-registry` даёт только упоминания в док-комментариях:
`crates/vibe-registry/src/index_client/wire.rs:36,70`;
`crates/vibe-registry/src/search/full_scan.rs:5`), т.е. клиентские view
парсят провод СВОИМИ типами (`crates/vibe-registry/src/index_client/wire.rs`)
на базе `vibe-core` (`crates/vibe-registry/src/index_client/wire.rs:12`).
Либо Ф4.2 выносит словари в общий листовой crate, либо `vibe-registry`
начинает зависеть от `vibe-index::types` (или наоборот, словарь каталога
переезжает в `vibe-core` рядом с существующим — тогда исчезает и дубль
`PackageKind`/`NamingConvention`/`DeliveryMode`).

### 4.2.1. BindingSite — известный дефект, правило расходится СЕГОДНЯ

- **Определения:** писатель `crates/vibe-index/src/index/inverted.rs:87-92`
  (`#[serde(rename_all = "kebab-case")]`), клиент
  `crates/vibe-registry/src/index_client/wire.rs:92-99`
  (`#[serde(rename_all = "lowercase")]`).
- **rename_all расходятся:** да — kebab-case против lowercase.
- **При каком значении расхождение стало бы видимым:** при любом будущем
  многословном варианте. Конкретно: вариант `SubSkill` писатель отправит
  строкой `"sub-skill"` (kebab вставляет дефис на границе регистра), клиент
  же ждёт `"subskill"` (lowercase только снижает регистр) —
  `serde_json::from_str::<BindingSite>("\"sub-skill\"")` на клиентской копии
  упадёт «unknown variant». Сегодня оба варианта однословные
  (`Package`, `Subskill`), поэтому строки совпадают по случайности.
- **Что должно стать одним определением:** копия писателя
  (`vibe-index::types`, рядом с `PurlRow`), реэкспортированная в
  `vibe-registry::index_client` вместо локальной. **Импортёры сегодня:**
  писательскую копию использует только `PurlRow.binding_site`
  (`crates/vibe-index/src/index/inverted.rs:84`) и тесты того же файла
  (`:431-433`); клиентская копия экспортирована из модуля
  (`crates/vibe-registry/src/index_client/mod.rs:29`) и используется в
  `crates/vibe-cli/src/commands/search/purl.rs:14,39` и тестах
  `crates/vibe-registry/tests/index_search.rs:21,273-275`.
- **Дополнительно (§8 п. 2):** те же строки `"package"`/`"subskill"`
  продублированы в третий раз литералами в HTTP-роуте
  `crates/vibe-index/src/server/routes/purls.rs:47-51` — он не использует
  ни одну из enum-копий.

### 4.2.2. PackageKind — дубль, правило совпадает, паритет-теста нет (U3)

- **Определения:** `crates/vibe-core/src/package_ref/kind.rs:29-31`
  (`#[serde(rename_all = "lowercase")]`, варианты Flow/Feat/Stack/Tool/Mcp/Lang,
  `:32-48`) и `crates/vibe-index/src/types/kinds.rs:16-21` (то же правило,
  `:21-34`; плюс clap `#[value(rename_all = "kebab-case")]` `:20` — это CLI,
  не провод).
- **rename_all расходятся:** нет — обе lowercase; строки провода идентичны
  (flow, feat, stack, tool, mcp, lang).
- **При каком значении расхождение стало бы видимым:** правила не
  различимы НИКАКИМ значением (доказательство: для однословных
  идентификаторов без внутренних заглавных kebab = lowercase, а все 6
  вариантов таковы). Видимым становится **дрейф множества**: новый вариант,
  добавленный в одну копию (пример: `"plugin"`), даст «unknown variant» на
  стороне, которая его не получила. Компилятор это ловит лишь наполовину —
  исчерпывающий конвертер `crates/vibe-index/src/scanner/manifest.rs:66-75`
  сломает сборку при новом варианте core-копии, но односторонний rename
  serde-атрибута пройдёт без всякого сигнала (паритет-теста нет, U3).
- **Одно определение после Ф4.2 + импортёры:** канонической должна стать
  копия `vibe-core` (её уже читает клиент: `SearchHit.kind` /
  `PurlLookupHit.kind` — `crates/vibe-registry/src/index_client/wire.rs:12,57,85`),
  а `vibe-index` — реэкспортировать её из `crate::types` (сегодня
  `vibe-index::types::PackageKind` используется внутри самого `vibe-index`:
  `crates/vibe-index/src/types/entry/mod.rs:53`,
  `crates/vibe-index/src/types/entry/relations.rs:18`,
  `crates/vibe-index/src/index/inverted.rs:67,76`,
  `crates/vibe-index/src/index/search.rs:19`,
  серверные строки `crates/vibe-index/src/server/routes/packages.rs:26`,
  `capabilities.rs:13`, `purls.rs:13`; извне crate `vibe_index::types`
  никто не импортирует — grep по `crates` вне `vibe-index` даёт ноль).
  Препятствие названо в `crates/vibe-index/src/types/kinds.rs:1-5` и
  `crates/vibe-index/src/scanner/manifest.rs:12-16`: копии существуют ради
  standalone-распространения `vibe-index` и derive'ов `Ord + ValueEnum`,
  которых нет у core-оригинала.

### 4.2.3. NamingConvention — дубль, правило совпадает, КОНВЕРТЕРА НЕТ ВООБЩЕ

- **Определения:** `crates/vibe-core/src/manifest/project.rs:270-293`
  (по-вариантные `#[serde(rename = …)]`: fqdn/kind-name/name/kind/name) и
  `crates/vibe-index/src/types/kinds.rs:87-112` (те же четыре rename,
  `:94,99,104,109`).
- **расходятся:** нет — правила и строки идентичны.
- **При каком значении видно:** как у PackageKind — не правилом (правила
  буквально одинаковы), а дрейфом множества: пятое значение или односторонний
  rename (скажем, core переименует `"kind/name"` в `"kind_slash_name"`)
  сломает чтение `repomd.json`/журнала на другой стороне. Отличие от
  PackageKind/DeliveryMode/SourceKind: **между копиями нет даже
  match-конвертера** (grep по `vibe-index` не находит преобразования
  core↔index NamingConvention; в `crates/vibe-index/src/cli/init.rs:34,54,72`
  значение приходит из clap прямо в index-копию), поэтому одностороннее
  изменение не поймает ни компилятор, ни тест — только замороженные строки
  селф-теста `crates/vibe-index/src/types/kinds.rs:170-187`, и то на своей
  копии.
- **Импортёры:** core-копия — конфигурация `[[registry]]` в `vibe.toml`
  (`crates/vibe-core/src/manifest/project.rs:271`, тесты
  `crates/vibe-registry/src/multi_registry_resolver/test_support.rs:18`,
  `walk/tests.rs:10`, `crates/vibe-registry/tests/index_fast_path.rs:22`);
  index-копия — `Repomd.naming` (`crates/vibe-index/src/types/repomd.rs:23`),
  `Event::Initialised.naming` (`crates/vibe-index/src/journal/record.rs:45`),
  CLI `init` (`crates/vibe-index/src/cli/init.rs:11,35`), проектор
  (`crates/vibe-index/src/journal/project.rs:26,81`). Мост между ними —
  только провод (строки), что и делает дубль опасным.

### 4.2.4. DeliveryMode — дубль, правило совпадает, конвертер есть

- **Определения:** `crates/vibe-core/src/manifest/subskill.rs:118-120`
  (`#[serde(rename_all = "kebab-case")]`, Eager/LazyPush/LazyPull) и
  `crates/vibe-index/src/types/entry/content.rs:49-51` (то же).
- **расходятся:** нет; строки eager/lazy-push/lazy-pull идентичны (у
  core-копии есть ещё `as_str` с теми же значениями:
  `crates/vibe-core/src/manifest/subskill.rs:132-137`).
- **При каком значении видно:** только дрейфом множества — новый вариант или
  односторонний rename (например `"lazy-push"` → `"push"` на одной стороне)
  даст unknown variant на другой. Новый вариант сломает сборку конвертера
  `crates/vibe-index/src/scanner/manifest.rs:183-188`; rename — пройдёт
  молча.
- **Импортёры:** core-копия — манифест subskill
  (`crates/vibe-core/src/manifest/subskill.rs:120`, конвертация на входе
  сканера `crates/vibe-index/src/scanner/manifest.rs:27`); index-копия —
  `SubskillEntry.delivery` (`crates/vibe-index/src/types/entry/content.rs:60`),
  т.е. каждая строка `primary.jsonl`/`by-name`/`by-purl` с subskills.

### 4.2.5. SourceKind — дубль, найден этим замером, правило совпадает

- **Определения:** `crates/vibe-core/src/manifest/lockfile.rs:197-199`
  (`#[serde(rename_all = "lowercase")]`, 6 вариантов Registry/Git/Override/
  Path/Embedded/Local, `:199-230`) и
  `crates/vibe-cli/src/commands/tree/model.rs:241-243` (Serialize-only,
  те же 6 вариантов, `:243-253`).
- **расходятся:** нет; строки registry/git/override/path/embedded/local
  идентичны (у CLI-копии свой `source_kind_label` с теми же строками:
  `crates/vibe-cli/src/commands/tree/tui/modal.rs:174-182`).
- **При каком значении видно:** дрейфом множества. Седьмой источник
  (например `"vendored"`) сломает сборку match-конвертера
  `crates/vibe-cli/src/commands/tree/build.rs:429-437`; односторонний rename
  — пройдёт молча (CLI-копия Serialize-only, обратного чтения нет, но её
  JSON-вывод `tree --json` перестанет совпадать со словарём lockfile).
- **Импортёры:** core-копия — lockfile `vibe.lock`
  (`crates/vibe-core/src/manifest/lockfile.rs:199`), читатель в
  `vibe-index` (`crates/vibe-index/src/lockfile.rs:22-27` — через
  index-`PackageKind`, не core); CLI-копия — JSON-модель дерева
  (`crates/vibe-cli/src/commands/tree/model.rs:72`, сборка
  `build.rs:429-437`).

## 5. Публикуемые поверхности и их ридеры (G11)

«Каталог пишет на диск» — всё, что пишет `vibe-index` в data-dir и что уходит
клиентам (git-репозиторий каталога + HTTP-роуты раздачи файлов
`crates/vibe-index/src/server/mod.rs:66-102`). Полный список писателей
получен grep'ом `atomic_write|write_to(` по `crates/vibe-index/src` — вне
`Index::write_to` пишут только `checkpoint::save`, `journal::store::append`
и `init` (два прозовых файла).

| файл на диске | константа (цитата) | писатель (цитата) | ридер ФАЙЛА | round-trip тест | тип строки/записи |
|---|---|---|---|---|---|
| `primary.jsonl` | `FILENAME = "primary.jsonl"` `crates/vibe-index/src/index/primary.rs:20` | `primary::write` `primary.rs:43` (вызов из `Index::write_to` `crates/vibe-index/src/index/memory.rs:216`) | **есть**: `primary::read` `primary.rs:75`, `parse` `:84` | есть: `round_trip_sorts_entries` `primary.rs:156`, `write_persists_on_disk` `:188` | `VersionEntry` — `crates/vibe-index/src/types/entry/mod.rs:50` |
| `primary.jsonl.gz` | `FILENAME_GZ = "primary.jsonl.gz"` `primary.rs:21` | `primary::write` → `gzip_deterministic` `primary.rs:51-53,65` | **НЕТ** — распаковка `gzip_round_trips_to_original_bytes` `primary.rs:214-221` гоняется по байтам в памяти, файл с диска никто не читает и не декодирует | частичный: `gzip_round_trips_to_original_bytes` `primary.rs:214` (байты, не файл); HTTP-форма — `primary_jsonl_gz_served_with_gzip_encoding` `crates/vibe-index/tests/server_e2e.rs:256` | те же `VersionEntry`, gzip-обёртка |
| `by-name/<name>.json` | `DIRNAME = "by-name"` `crates/vibe-index/src/index/by_name.rs:24` | `by_name::write` `by_name.rs:46` (из `Index::write_to` `memory.rs:250`) | **есть**: `by_name::read` `by_name.rs:57`, `read_all` `:72` | есть: `round_trip_through_disk` `by_name.rs:187` | `NameEntry` — `crates/vibe-index/src/types/entry/aggregate.rs:80` |
| `by-cap/<slug>.jsonl` | `BY_CAP_DIRNAME = "by-cap"` `crates/vibe-index/src/index/inverted.rs:35` | `inverted::write_capability` `inverted.rs:200` (из `Index::write_to` `memory.rs:265`) | **НЕТ** — в `inverted.rs` только `entry_count_capability` `:241` (подсчёт файлов); типизированного читателя строк нет нигде (grep §9) | НЕТ типизированного: `write_capability_round_trips_on_disk` `inverted.rs:437` проверяет подстроку в сыром тексте, не `from_str::<CapabilityRow>`; HTTP-тест — подстроки в теле `crates/vibe-index/tests/server_e2e.rs:207-221` | `CapabilityRow` — `inverted.rs:66` |
| `by-purl/<slug>.jsonl` | `BY_PURL_DIRNAME = "by-purl"` `inverted.rs:36` | `inverted::write_purl` `inverted.rs:215` (из `Index::write_to` `memory.rs:272`) | **НЕТ** — только `entry_count_purl` `:245` | НЕТ даже подстрочного дискового: в `inverted.rs` тестов на `write_purl` нет; HTTP-тест — подстроки `server_e2e.rs:223-240` | `PurlRow` — `inverted.rs:75` |
| `repomd.json` | `FILENAME = "repomd.json"` `crates/vibe-index/src/index/repomd.rs:14` | `repomd::write` `repomd.rs:27` (последним, из `Index::write_to` `memory.rs:302`) | **есть**: `repomd::read` `repomd.rs:32`, `exists` `:41` | есть: `round_trips_on_disk` `repomd.rs:75` | `Repomd` — `crates/vibe-index/src/types/repomd.rs:19` |
| `state/journal/<YYYY>-<MM>.ndjson` | имя шарда `shard_name` `crates/vibe-index/src/journal/store.rs:35-37`; каталог `default_dir` `:29-31` | `journal::store::append` `store.rs:43` | **есть**: `journal::store::replay` `store.rs:64` | есть: `every_event_variant_survives_round_trip` `crates/vibe-index/src/journal/tests.rs:197` | `JournalRecord`/`Event` — `crates/vibe-index/src/journal/record.rs:22,34` |
| `state/checkpoint.json` | `FILENAME = "checkpoint.json"` `crates/vibe-index/src/index/checkpoint.rs:16` | `checkpoint::save` `checkpoint.rs:72` | **есть**: `checkpoint::load` `checkpoint.rs:55` | есть: `save_then_load_round_trips` `checkpoint.rs:98` | `Checkpoint` — `checkpoint.rs:19` |
| `README.md`, `.gitignore` (в data-dir) | литералы в `crates/vibe-index/src/cli/init.rs:90,107` | `write_gitignore` `init.rs:89`, `write_readme` `init.rs:106` | нет (проза для человека) | нет | нет типа — G11 здесь не про них, но в таблице для полноты |

**Прямой ответ: ридера файла сегодня нет у трёх публикуемых поверхностей —
`by-cap/<slug>.jsonl`, `by-purl/<slug>.jsonl` и `primary.jsonl.gz`.**
`state/**` — не публикуется: журнал и чекпоинт живут под gitignore
(`init.rs:94-99` «everything under state/ … stays out of the source tree»),
в `repomd.json` не попадают (тип `Repomd` описывает файлы «beneath the data
directory (excluding `state/`)» — `crates/vibe-index/src/types/repomd.rs:29-33`)
и HTTP их не раздаёт (роуты `server/mod.rs:66-102` перечисляют только шесть
файлов каталога).

**Отдельно про HTTP.** Раздача файлов: `repomd.json`, `primary.jsonl`,
`primary.jsonl.gz`, `by-cap/<slug>.jsonl`, `by-purl/<slug>.jsonl`,
`by-name/<name>.json` — вербатим с диска
(`crates/vibe-index/src/server/routes/index_files.rs:17-117`). «Ридер HTTP-
ответа» и «ридер файла» — действительно разные вещи, и они не путаются:
клиентские view живут в `crates/vibe-registry/src/index_client/wire.rs` —
`NameEntryView` (`:17-33`, читает HTTP-ответ `by-name` и терпит лишние
поля), `SearchResults`/`SearchHit` (`:44-67`, роут `/v1/packages`),
`PurlLookupResults`/`PurlLookupHit` (`:71-89`, роут `/v1/purls/{purl}`);
это НЕ читатели файлов на диске. По HTTP клиент имеет ридеры для: by-name
(`IndexClient::list_versions`/`name_candidates` —
`crates/vibe-registry/src/index_client/mod.rs:234,289`), purls
(`lookup_purl` `:337`), поиска (`search` `:395`). **У HTTP-поверхностей
`by-cap/<slug>.jsonl`, `by-purl/<slug>.jsonl` (файловые роуты) и
`primary.jsonl`/`primary.jsonl.gz` клиентского ридера нет вовсе** — их
никто не скачивает; возможности вместо этого обслуживает живой роут
`/v1/capabilities/{capability}` (`crates/vibe-index/src/server/routes/capabilities.rs:32`),
у которого, в свою очередь, нет клиентского view в `IndexClient` (в
`crates/vibe-registry/src/index_client/mod.rs:50` он упомянут только в
док-комментарии). Итог G11-картины: файловые ридеры есть у 3 из 6 раздаваемых
файлов, HTTP-view — у 2 живых роутов из 3 (+by-name, который и файл, и роут).

## 6. Словарь событий журнала — форма с кода

Тип: `Event` в `crates/vibe-index/src/journal/record.rs:32-110`;
обёртка `JournalRecord` — `record.rs:21-26`.

- **Сколько вариантов: 11** (подсчёт командой §9: `11`; имена — Initialised,
  Published, Frozen, Yanked, Removed, Renamed, Notice, ChannelSet,
  ChannelUnset, ForceReplaced, EntrySetReplaced).
- **Тегирование перечисления в целом** — `record.rs:33`:
  `#[serde(tag = "kind", rename_all = "snake_case")]` — смежно-тегированная
  (adjacently tagged) форма: `{"kind":"<тег>", …поля…}`.
- **Обёртка записи целиком** — `record.rs:22-26`: `at: chrono::DateTime<Utc>`
  (на проводе RFC-3339 строка), `actor: String`, `event: Event`. Имена полей
  на проводе: `at`, `actor`, `event`.
- **По вариантам** (имя → тег на проводе → поля с типами; опциональность —
  по serde-атрибутам варианта):

| вариант | тег | поля |
|---|---|---|
| Initialised | `initialised` | `registry: String`, `registry_url: String`, `naming: NamingConvention` (`record.rs:42-46`) |
| Published | `published` | `entry: Box<VersionEntry>` — на проводе просто вложенный объект `VersionEntry` (`record.rs:53`) |
| Frozen | `frozen` | `group: Group`, `name: String`, `version: Version`, `content_hash: String` (`record.rs:58-63`) |
| Yanked | `yanked` | `group: Group`, `name: String`, `version: Version`, `reason: String` (`record.rs:64-69`) |
| Removed | `removed` | `group: Group`, `name: String`, `version: Option<Version>` — `None` = удалить весь пакет (`record.rs:70-74`) |
| Renamed | `renamed` | `from: (Group, String)`, `to: (Group, String)` — на проводе двухэлементные массивы (`record.rs:75-78`) |
| Notice | `notice` | `group: Group`, `name: String`, `text: String` (`record.rs:79-83`) |
| ChannelSet | `channel_set` | `group: Group`, `name: String`, `channel: String`, `version: Version` (`record.rs:84-89`) |
| ChannelUnset | `channel_unset` | `group: Group`, `name: String`, `channel: String` (`record.rs:90-94`) |
| ForceReplaced | `force_replaced` | `group: Group`, `name: String`, `version: Version`, `old_hash: String`, `new_hash: String`, `reason: String` (`record.rs:95-102`) |
| EntrySetReplaced | `entry_set_replaced` | `source: String` (`record.rs:109`) |

  Ни один вариант не несёт `skip_serializing_if`/`default` на полях — все
  поля обязательны; единственная опциональность смысловая: `version:
  Option<Version>` у `Removed` сериализуется как `null`, когда пакет удаляется
  целиком.
- **Проекция:** проецируются 6 — Initialised, Published, Removed,
  EntrySetReplaced, Yanked, Frozen (`crates/vibe-index/src/journal/project.rs:85-134`);
  **отказываются по имени 5** — Renamed, Notice, ChannelSet, ChannelUnset,
  ForceReplaced (`project.rs:135-149`), каждый через
  `unprojectable(variant, carrier)` (`project.rs:205-211`), где причина
  названа текстом-носителем: «package rename», «per-package notice»,
  «channels», «channels», «forced content replacement». Отказ — это
  `Error::Unprojectable` с сообщением «the journal holds a \`{variant}\`
  record, but its carrier ({carrier}) is not built in this vibe-index —
  skipping the record would project a catalog the journal does not describe».
- **Боксирование:** один вариант — `Published { entry: Box<VersionEntry> }`
  (`record.rs:53`); на проводе НЕ ВИДНО (док-комментарий `record.rs:47-53`:
  serde рендерит `Box<T>` как `T`), бокс — чисто мера против раздувания
  памяти `Vec<JournalRecord>` в `replay`.
- Все 11 вариантов проходят round-trip через append/replay — тест
  `every_event_variant_survives_round_trip`
  `crates/vibe-index/src/journal/tests.rs:197-296`.

## 7. Инвентарь типов под шесть схем Ф4.1

Общие листовые типы для всех шести схем: `Version` (semver, на проводе
строка), `DateTime<Utc>` (chrono, RFC-3339 строка), `Group`
(`crates/vibe-core/src/package_ref.rs:43-45` — newtype над `String`,
на проводе голая строка), примитивы. Читательская терпимость (PROP-044 §4.4):
неизвестные поля читаются и игнорируются — док-комментарий
`crates/vibe-index/src/types/entry/mod.rs:37-44`.

### 7.1. `entry.jtd.json` → `VersionEntry`

Корень: `crates/vibe-index/src/types/entry/mod.rs:50` (декларация `:45-149`).
Поля с типами и serde-атрибутами:

- `schema_version: u32` (`:51`)
- `kind: PackageKind` (`:53`; словарь №43, lowercase)
- `group: Group` (`:58`)
- `name: String` (`:59`)
- `version: Version` (`:60`)
- `content_hash: String`, `source_url: String`, `source_ref: String` (`:62-64`)
- `resolved_commit: Option<String>` — `#[serde(default, skip_serializing_if = "Option::is_none")]` (`:66-67`)
- `registry: String` (`:69`)
- `workspace_origin: Option<WorkspaceOriginEntry>` — default/skip-none (`:74-75`)
- `license: Option<String>` — default/skip-none (`:77-78`)
- `authors: Vec<String>` — default/skip-empty (`:79-80`)
- `description: Option<String>` — default/skip-none (`:81-82`)
- `homepage: Option<String>` — default/skip-none (`:83-84`)
- `keywords: Vec<String>` — default/skip-empty (`:85-86`)
- `describes: Option<String>` — default/skip-none (`:91-92`)
- `compatibility: CompatibilityEntry` — default/skip-`is_empty` (`:94-95`)
- `provides: ProvidesEntry` — default/skip-empty (`:97-98`)
- `requires: RequiresEntry` — default/skip-empty (`:100-101`)
- `requires_any: Vec<RequiresAnyEntry>` — default/skip-empty (`:103-104`)
- `obsoletes: ObsoletesEntry` — default/skip-empty (`:106-107`)
- `conflicts: ConflictsEntry` — default/skip-empty (`:109-110`)
- `features: FeaturesEntry` — default/skip-empty (`:112-113`)
- `subskills: Vec<SubskillEntry>` — default/skip-empty (`:115-116`)
- `i18n: I18nEntry` — default/skip-empty (`:118-119`)
- `boot_snippet: Option<BootSnippetEntry>` — default/skip-none (`:121-122`)
- `files_count: u32` (`:124`)
- `must_understand: Vec<String>` — default/skip-empty (`:131-132`)
- `yanked: bool` — default/skip-`is_false` (`:136-137`; helper `crates/vibe-index/src/types/mod.rs:26-28`)
- `frozen: bool` — default/skip-`is_false` (`:144-145`)
- `indexed_at: DateTime<Utc>` (`:147`), `indexed_by: String` (`:148`)

Вложенные типы (транзитивно до листьев):

- `PackageKind` — `crates/vibe-index/src/types/kinds.rs:21` (второе определение `crates/vibe-core/src/package_ref/kind.rs:31`)
- `Group` — `crates/vibe-core/src/package_ref.rs:45`
- `WorkspaceOriginEntry` — `crates/vibe-index/src/types/entry/content.rs:18` (поля `upstream: String`, `path: String`, `commit: Option<String>` skip-none `:24-25`, `generated_by: String`, `generated_at: String`)
- `CompatibilityEntry` — `crates/vibe-index/src/types/entry/relations.rs:14` (`min_vibe_version: Option<String>` skip-none `:15-16`, `requires_kinds: Vec<PackageKind>` skip-empty `:17-18` — снова `PackageKind`)
- `ProvidesEntry` — `relations.rs:28` (`capabilities: Vec<String>`)
- `RequiresEntry` — `relations.rs:40` (`packages: Vec<String>`, `capabilities: Vec<String>`, оба skip-empty)
- `RequiresAnyEntry` — `relations.rs:54` (`one_of: Vec<String>`)
- `ObsoletesEntry` — `relations.rs:59` (`packages: Vec<String>`)
- `ConflictsEntry` — `relations.rs:71` (`packages: Vec<String>`)
- `FeaturesEntry` — `content.rs:36` (`features: BTreeMap<String, Vec<String>>`, `exclusive: BTreeMap<String, Vec<String>>`, оба skip-empty)
- `SubskillEntry` — `content.rs:58` (`path: String`, `delivery: DeliveryMode`, `describes: Option<String>` skip-none, `description: Option<String>` skip-none, `channels: Vec<String>` skip-empty)
- `DeliveryMode` — `content.rs:51` (второе определение `crates/vibe-core/src/manifest/subskill.rs:120`)
- `I18nEntry` — `content.rs:70` (`available: Vec<String>` skip-empty, `default: Option<String>` skip-none)
- `BootSnippetEntry` — `content.rs:88` (`source: String`, `category: Option<String>` skip-none)

**Уже описаны существующей схемой:** `PackageKind` — да, как `package_kind`
в `schemas/list_report.jtd.json:34-42` и
`schemas/registry_sync_report.jtd.json:33-42` (уже ДВЕ копии в схемах —
прямая угроза G9: `entry.jtd.json` станет третьей). Остальные — нет.

### 7.2. `repomd.jtd.json` → `Repomd`

Корень: `crates/vibe-index/src/types/repomd.rs:19` (декларация `:14-35`).
Поля: `schema_version: u32`, `registry: String`, `registry_url: String`,
`naming: NamingConvention`, `generated_at: DateTime<Utc>`,
`generator: String`, `package_count: u32`, `version_count: u32`,
`files: BTreeMap<String, RepomdFileEntry>` — все без serde-атрибутов
(обязательные). `SCHEMA_VERSION = 1` — `repomd.rs:38`.

Вложенные: `NamingConvention` — `crates/vibe-index/src/types/kinds.rs:88`
(второе определение `crates/vibe-core/src/manifest/project.rs:271`);
`RepomdFileEntry` — `repomd.rs:43` (тегированное объединение
`#[serde(tag = "kind", rename_all = "lowercase")]` `:42`; вариант
`Directory { entries: u32 }` → `{"kind":"directory","entries":N}`,
вариант `File { size: u64, sha256: String }` → `{"kind":"file","size":N,"sha256":"…"}`).

**В существующих схемах:** не описаны. Угроза G9 — `NamingConvention`
окажется в двух новых схемах сразу (`repomd.jtd` и `journal.jtd`, см. 7.6).

### 7.3. `by_name.jtd.json` → `NameEntry`

Корень: `crates/vibe-index/src/types/entry/aggregate.rs:80` (декларация
`:75-93`). Поля: `name: String`, `indexed_at: DateTime<Utc>`,
`packages: Vec<PackageEntry>`, `tombstone: Option<Tombstone>` —
default/skip-none (`:91-92`).

Вложенные: `PackageEntry` — `aggregate.rs:25` (`group: Group`, `name: String`,
`indexed_at: DateTime<Utc>`, `latest_stable: Option<Version>` default/skip-none
`:29-30`, `versions: Vec<VersionEntry>`); `Tombstone` — `aggregate.rs:62`
(`reason: String`, `superseded_by: Option<String>` default/skip-none `:66-67`);
плюс ВСЁ дерево `VersionEntry` из 7.1.

**В существующих схемах:** нет. Угроза G9: `VersionEntry` (и его
`package_kind`) понадобится и здесь, и в `entry.jtd`, и в `journal.jtd` —
три корня, одно поддерево.

### 7.4. `by_cap.jtd.json` → `CapabilityRow`

Корень: `crates/vibe-index/src/index/inverted.rs:66` (декларация `:65-72`).
Поля: `kind: PackageKind`, `group: Group`, `name: String`,
`version: Version`, `capability: String` — все обязательные, атрибутов нет.
Вложенные: `PackageKind`, `Group`, `Version`. Сортировка строк файла — по
`(group, name, version, capability)` (`inverted.rs:146-155`).

**В существующих схемах:** `PackageKind` — снова `package_kind` из
`list_report`/`registry_sync_report` (третья-четвёртая копия угрозы).

### 7.5. `by_purl.jtd.json` → `PurlRow`

Корень: `crates/vibe-index/src/index/inverted.rs:75` (декларация `:74-85`).
Поля: `kind: PackageKind`, `group: Group`, `name: String`,
`version: Version`, `purl: String`, `binding_site: BindingSite` — все
обязательные. Вложенные: `PackageKind`, `Group`, `Version`, и
**`BindingSite` — та самая больная копия** (`inverted.rs:89`, kebab; клиентская
`lowercase` — `crates/vibe-registry/src/index_client/wire.rs:94`): схема
обязана зафиксировать ОДНО правило до того, как опишет этот файл.
Сортировка — `(group, name, version, binding_site)` (`inverted.rs:156-165`).

**В существующих схемах:** нет.

### 7.6. `journal.jtd.json` → `JournalRecord` (+ `Event`)

Корень: `crates/vibe-index/src/journal/record.rs:22` (декларация `:21-26`):
`at: DateTime<Utc>`, `actor: String`, `event: Event`. `Event` —
`record.rs:34` с формой всех 11 вариантов из §6. Вложенные транзитивно:
`NamingConvention` (`types/kinds.rs:88`), `Group`, `Version`, и ВСЁ дерево
`VersionEntry` (через `Published { entry: Box<VersionEntry> }` —
`record.rs:53`; бокс на проводе невидим).

**В существующих схемах:** нет. Угроза G9 здесь тройная: `NamingConvention`
дублируется с `repomd.jtd`, `VersionEntry`-поддерево — с `entry.jtd`/
`by_name.jtd`, и теги вариантов (`initialised`…`entry_set_replaced`)
нигде больше не живут — их словарь уникален для этой схемы.

### 7.7. Сводка G9-угроз по шести схемам

Словарь `package_kind` (6 значений) после Ф4.1 будет нужен в:
`entry.jtd`, `by_name.jtd` (транзитивно), `by_cap.jtd`, `by_purl.jtd`,
`journal.jtd` (транзитивно) — плюс уже существует в `list_report.jtd.json`
и `registry_sync_report.jtd.json`. Чистый JTD не имеет межфайловых ссылок
(definitions локальны файлу), значит без решения уровня генерации
(общий источник → автогенерация вставок) Ф4.1 умножит копию до семи.
`delivery` (3 значения) нужен в `entry.jtd`/`by_name.jtd`/`journal.jtd`;
`naming` (4 значения) — в `repomd.jtd`/`journal.jtd`; `binding_site`
(2 значения) — в `by_purl.jtd` (и в HTTP-view `/v1/purls` — строки
`crates/vibe-index/src/server/routes/purls.rs:47-51`).

## 8. Дыры и неожиданности

1. **Заявленный паритет-тест `PackageKind` не существует.** Док-комментарий
   `crates/vibe-index/src/types/kinds.rs:4-5` («parity-test it (slice 3)
   against `vibe-core` to catch divergence at CI time») и `:27-28` («the
   parity test below keeps the copies honest») обещают тест, которого в
   дереве нет: `kinds.rs:148-167` тестирует только свою копию, а единственный
   «parity»-файл `crates/vibe-index/tests/content_hash_parity.rs:104-138`
   сверяет хеши контента. Комментарий лжёт о защите — хуже, чем её отсутствие.
2. **У `BindingSite` три копии словаря, не две:** помимо enum'ов
   (`crates/vibe-index/src/index/inverted.rs:89` kebab и
   `crates/vibe-registry/src/index_client/wire.rs:94` lowercase) те же строки
   зашиты литералами в HTTP-роуте — `crates/vibe-index/src/server/routes/purls.rs:47-51`
   (`"package"` / `"subskill"` как `&'static str`, минуя оба enum'а).
3. **Словарные омонимы с разными именами типов:** `LinkType`
   (`crates/vibe-core/src/manifest/package.rs:418`, static/dynamic/
   static-transitive/static-hard) и `DeclaredLink`
   (`crates/vibe-cli/src/commands/tree/model.rs:206`, те же четыре строки) —
   один словарь под двумя именами; рядом `LoadType`/`IndexKind`
   (`tree/model.rs:197,235`) дублируют его подмножество static/dynamic.
   В перепись §3 не попали как «два определения одного типа», но для Ф4.2
   это та же болезнь.
4. **`package_kind` уже продублирован в существующих схемах** —
   `schemas/list_report.jtd.json:34-42` и
   `schemas/registry_sync_report.jtd.json:33-42`: G9 нарушен в схемном слое
   ещё ДО Ф4.1, и шесть новых схем (§7.7) умножат копию до семи, если
   механизм шаринга не решить.
5. **`primary.jsonl.gz` — третья поверхность без ридера файла** (не
   названная в U2): пишется всегда (`crates/vibe-index/src/index/primary.rs:51-57`),
   раздаётся HTTP (`server/routes/index_files.rs:31-69`), а декодер-тест
   `gzip_round_trips_to_original_bytes` (`primary.rs:214-221`) гоняется по
   байтам в памяти — файл с диска не читает никто.
6. **Боевой путь загрузки каталога не читает `primary.jsonl` и инвертированные
   индексы:** `Index::load_from` (`crates/vibe-index/src/index/memory.rs:315-317`)
   читает только `repomd.json` + `by-name/*.json`; серверное состояние при
   мутациях строится повторной свёрткой журнала (`packages.rs:401-435`).
   Ридер `primary.jsonl` существует (`primary.rs:75`), но в загрузке не
   участвует — его round-trip охраняет только юнит-тест.
7. **У `NamingConvention` нет конвертера между копиями** — в отличие от
   `PackageKind`/`DeliveryMode`/`SourceKind`, между core- и index-копиями
   нет match-моста (grep по `crates/vibe-index` не находит преобразования;
   `cli/init.rs:34,54,72` наполняет index-копию из clap). Односторонний
   rename строки не поймает ни компилятор, ни какой-либо тест.
8. **`RecipeId` — шестой кодовый дубль вне переписи:** два определения без
   serde (`crates/vibe-index/src/hash_recipe.rs:27`,
   `crates/vibe-registry/src/hash_recipe.rs:27`), чей словарь виден на
   проводе в префиксах хешей (`sha256:`, `sha256-tree/1:`), держится
   паритет-тестом хешей (`crates/vibe-index/tests/content_hash_parity.rs:124-138`).
9. **Двусмысленные строки провода от правил:** `WalkAttemptStatus::Public401`
   под kebab-case даёт `public401` без дефиса (цифровая граница не
   разделяется) — `crates/vibe-registry/src/multi_registry_resolver/attempt.rs:26-33`;
   `FoldLoss::Actionstage` под lowercase даёт `actionstage` —
   `crates/progress-core/src/rollup.rs:70-77`. Для §3 эти значения выведены
   из правила; cargo-проверка невозможна (запрещена заданием).
10. **Словари без rename-правила едут идентификаторами Rust:**
    `Capability` (`crates/vibe-actions/src/action.rs:147` — Safe/Mutating/
    Dangerous), `Affinity` (`crates/vibe-mcp/src/agentic.rs:62` —
    AgenticOnly/StandaloneOnly/Both): любой рефакторинг имени варианта молча
    меняет провод.
11. **HTTP-роут `/v1/capabilities/{capability}` не имеет клиентского view**
    в `IndexClient` (упомянут только в док-комментарии
    `crates/vibe-registry/src/index_client/mod.rs:50`), при этом файловый
    роут `by-cap` тоже никем не читается — CapabilityRow доступен потребителю
    только как сырой JSON.
12. **PROP-043-журнал ведёт себя как реестровый, но словарем не связан:**
    `progress_core::journal::Event` — тоже `tag = "kind"` + kebab
    (`crates/progress-core/src/journal.rs:18`), тезка реестрового `Event`
    (`record.rs:29-31` прямо предостерегает от путаницы). Для Ф4.1 не нужен,
    но при генерации схем имена тегов двух журналов лучше не смешивать.

## 9. Как воспроизвести этот замер

Только читающие команды; пути от корня рабочего дерева
(`.wt/F4-VOCAB`). Bash-семантика.

1. **Карта периметра и полная перепись enum:**
   ```sh
   find crates -name "*.rs" -not -path "*generated*" | wc -l   # → 602 файла
   grep -rn --include="*.rs" -E "^\s*(pub\s+)?enum\s+[A-Z]" crates | grep -v generated | wc -l   # → 229
   ```
2. **Кандидаты переписи (serde-derive в окне 8 строк над enum):**
   ```sh
   find crates -name "*.rs" -not -path "*generated*" > /tmp/rsfiles.txt
   perl -e 'use strict;use warnings;open my $fl,"<","/tmp/rsfiles.txt"or die;
   while(my $f=<$fl>){chomp $f;open my $fh,"<",$f or next;my@buf;my$ln=0;
   while(my $line=<$fh>){$ln++;
   if($line=~/^\s*(?:pub(?:\([^)]*\))?\s+)?enum\s+([A-Z]\w*)/){my$name=$1;
   my$ctx=join("",@buf);
   if($ctx=~/derive\s*\(/&&$ctx=~/Serialize|Deserialize/){print"$f:$ln:$name\n";}}
   push@buf,$line;shift@buf while@buf>8;}close$fh;}' | wc -l   # → 63
   ```
   Ручной отсев (ложные срабатывания окна и enum'ы с полями — список в §3)
   даёт **50 словарей**.
3. **Правила имён и по-вариантные переименования:**
   ```sh
   grep -rn --include="*.rs" "rename_all" crates | grep -v generated
   grep -rn --include="*.rs" "#\[serde(rename = " crates | grep -v generated
   ```
4. **Дубли имени типа (кандидаты больных словарей):** из списка п.2 взять
   третье поле и найти повторы (например `sort -t: -k3 | uniq -d -f2` после
   нормализации, либо глазами — 63 строки); повторы: BindingSite,
   PackageKind, NamingConvention, DeliveryMode, SourceKind (+RecipeId вне
   переписи).
5. **Паритет-тесты:** `grep -rn --include="*.rs" -i parity crates | grep -v generated`
   — единственный тестовый файл `crates/vibe-index/tests/content_hash_parity.rs`
   (хеши, не словари); сверка «импортирует ли кто-то обе копии»:
   `grep -rn "vibe_core::PackageKind" crates/vibe-index` → только конвертер
   `scanner/manifest.rs:23,66-75`.
6. **Публикуемые поверхности:** писатели —
   `grep -rn "atomic_write\|write_to(" crates/vibe-index/src`;
   ридеры — читать соответствующие модули (`index/primary.rs`,
   `index/by_name.rs`, `index/inverted.rs`, `index/repomd.rs`,
   `journal/store.rs`, `index/checkpoint.rs`) и искать функции чтения;
   HTTP-раздача — `crates/vibe-index/src/server/mod.rs:66-102` +
   `server/routes/index_files.rs`.
7. **Варианты события журнала (вывод дословно):**
   ```sh
   sed -n '/pub enum Event/,/^}/p' crates/vibe-index/src/journal/record.rs | grep -cE "^    [A-Z][A-Za-z]* ?(\{|,)"
   # → 11
   sed -n '/pub enum Event/,/^}/p' crates/vibe-index/src/journal/record.rs | grep -oE "^    [A-Z][A-Za-z]*" | tr -d ' '
   # → Initialised Published Frozen Yanked Removed Renamed Notice ChannelSet ChannelUnset ForceReplaced EntrySetReplaced
   ```
8. **Существующие схемы:** `ls schemas/` → 7 JTD-файлов;
   `grep -l "\"enum\"" schemas/*.json` → init_report, list_report,
   registry_sync_report (в двух последних — по своей копии `package_kind`).
