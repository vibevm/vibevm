# Смертность плана — карта домов, сегмент C (ТЗ, строки 239–1720)

## Метод

Сегмент 239–1720 прочитан целиком; рулинги перечислены по заголовкам `^\*\*Р<число>\.` (в
сегменте их 30, нумерация перезапускается в блоках — ссылки даю как `<строка>: Р<номер>`).
Спек-кандидат №1 (`vibevm/vibespecs/common/PROP-044-change-native-formats.xml`) прочитан полностью,
остальные спеки и весь код обхожу грепом по ключевому слову рулинга (Grep по `spec/`,
`crates/`, `xtask/`, `formats/`, `schemas/`, `tools/`), затем читаю найденный файл в месте
попадания. Цитатой считаю дословный текст докблока/комментария/аннотации схемы; класс
присваиваю по трём проверкам §5 (утверждение + причина + находимость без плана): `spec` —
@fact-якорь PROP-044 несёт утверждение/закон рулинга; `code` — докблок или аннотация
несёт и решение, и причину, а спека — нет; `both` — спека несёт несущую половину (закон,
который рулинг применяет), код — решение и механику; `none` — ни там, ни там. Дерево
заметно ушло за конец сегмента (Ф4.2c посажена: `types/entry/mod.rs` — реэкспортный шов),
поэтому «сегодняшний дом» проверялся по текущему дереву, а не по состоянию на момент
рулинга.

## Коммиты посадки по фазам

| фаза / подшаг | пометка в плане (строка) | коммиты |
|---|---|---|
| Фаза 0 (Ф0.1–Ф0.3) | «БЕЗ коммитов» (239); «Спайки не оставляют изменений дерева» (274) | — (находки `harvest/f0-rmw-volume.md`, `f0-gen-poc.md`, `f0-format-inventory.md`) |
| Ф1.1 Реестр форматов | «РЕШЕНЫ здесь 2026-08-14» (299); пометки СЕЛА нет | — (хэшей секция не называет) |
| Ф1.2 Эпоха в `vibe.toml` | «СЕЛА 2026-08-14» (327) | — |
| Ф1.3 Рецепт хэша | «СЕЛА 2026-08-14» (346) | — |
| Ф1.4 Слоты | «СЕЛА 2026-08-14» (366) | — |
| Ф1.5 Тегированное объединение | «СЕЛА 2026-08-14 — ФАЗА 1 ЗАКРЫТА» (545) | — |
| Ф2.1 Часы как вход | «СЕЛА 2026-08-14» (575) | — |
| Ф2.2 Версию не затирать | «СЕЛА 2026-08-14» (617) | — |
| Ф2.3 Идемпотентный upsert | «СЕЛА 2026-08-14 — ФАЗА 2 ЗАКРЫТА» (623) | — |
| Ф3 (фаза) | «ФАЗА ЗАКРЫТА 2026-08-14» (648) | спек-диффы `c2132dd0`, `4c977582` (652) |
| Ф3.1 Журнал фактов | «СЕЛА 2026-08-14» (678) | `8ba101d1` (650, 678) |
| Ф3.2a `init` кладёт первую запись | «Посадки: …» (650) | `64be15a8` |
| Ф3.2b проектор | (651) | `0c9ca4e0` |
| Ф3.2c1 переделка мутаций | (651) | `66c58f64` |
| Ф3.2c2 | (651) | `7a72c14f` |
| Ф3.2c3 | (651) | `f157a997` |
| Ф3.2d `xtask rebuild --check` | (651) | `c896e218` |
| Ф3.3 Строгость читателя | (652) | `dd3a1809` |
| Ф4.0 Генератор видит подкаталоги | «СЕЛА 2026-08-15 — три коммита» (1032) | `9c1ce20d`, `33b9ce59`, `f1ba88e0` |
| Ф4.2a боксирование (решение) | «решение босса 2026-08-15» (1144) | — (хэша в сегменте нет) |
| Ф4.1a словник и подстановка | «СЕЛА 2026-08-15» (1081) | `c7faa208` |
| Ф4.1a-2 транзитивная подстановка | «СЕЛА 2026-08-15» (1110) | `71e94e8f` |
| Ф4.1b-1 запись версии и оракул | отдельного хэша нет | — |
| Ф4.1b-2 остальные пять схем | «СЕЛИ 2026-08-15» (1132) | `b000e51f` (`by_name`,`by_cap`,`by_purl`), `456c4ea9` (`repomd`) |
| Ф4.1b-3 схема журнала | «СЕЛА 2026-08-15» (1160) | `e0a2248c` |
| Ф4.1 (итог) | «⇒ Ф4.1 ЗАКРЫТА: шесть схем из шести» (1176) | — |
| Ф4.2 сведение пина генератора | «СВЕДЕНО 2026-08-17» (1260) | `e8d8238f` (1282) |
| Ф4.2b-0 раскол `codegen/mod.rs` | «СЕЛА 2026-08-16» (1513) | `440c45fc` |
| Ф4.2b-1 открытие словаря | «СЕЛА 2026-08-17» (1521) | `50b0aa35` |
| Ф4.2b-2 snake_case | «СЕЛА 2026-08-17» (1536) | `21b0ac94` |
| Ф4.2b-3 `HashMap`→`BTreeMap` | «СЕЛА 2026-08-17» (1546) | `17193dd8` |
| Ф4.2b-4 политика пустого | «СЕЛА 2026-08-17» (1559) | `ab9edff6` |
| Ф4.2b-5 `x-default` + снятие `Box` | «СЕЛА 2026-08-17» (1572) | `e4b46885` (+ попутный `351a9594`, 1588) |
| Ф4.2b-6 `deny_unknown_fields` по реестру | «СЕЛА 2026-08-17» (1591) | `0fd7ce2d` |
| Ф4.2b-7 доменные типы | «СЕЛА 2026-08-17 … БЛОК Ф4.2b ЗАКРЫТ, семь шагов из семи» (1604–1605) | `c2c240b3` |

## Карта домов — по одной строке на рулинг

| строка | рулинг (первые ~8 слов) | класс | дом (якорь спеки / file:line) |
|---|---|---|---|
| 394 | [Ф1.4] Р1: Форма `yanked`/`frozen` — `bool` плюс именованный предикат | both | spec: PROP-044 #laws @fact:TERMS-SNAPSHOT-FROZEN-CHANNEL («one boolean axis … no third state»); code: `crates/vibe-core/src/manifest/package.rs:95-105` + `formats/vocabularies.json:451-462` |
| 424 | [Ф1.4] Р2: `must_understand` — фильтр над разобранными записями, не в парсере | code | `crates/vibe-index/src/index/memory.rs:334-345` + `crates/vibe-index/src/index/quarantine.rs:20-28` |
| 445 | [Ф1.4] Р3: Пропуск не молчит, форму выбирают, а не копируют | code | `crates/vibe-index/src/main.rs:58-70` |
| 473 | [Ф1.4] Р4: Одиннадцать литералов правятся поимённо | none | см. подраздел ниже |
| 480 | [Ф1.4] Р5: Проекция флага вписывается в ДВА места | code | `crates/vibe-index/src/cli/add.rs:183-185` + `crates/vibe-index/src/scanner/org_walk.rs:295-298` |
| 493 | [Ф1.4] Р6: `docs/format.md` правится тем же шагом, с тремя ложами | code | `crates/vibe-index/docs/format.md:72,132,177-178` |
| 502 | [Ф1.4] Р7: Перелом для старого бинаря — назван, а не обнаружен | spec | PROP-044 #machinery @fact:M-EPOCHS («Light breaks … need no epoch at all») + #obligations @fact:THE-PUBLIC-SWITCH («break notes are optional records») |
| 918 | [Ф4] Р1: Шаг Ф4.0 заводится ПЕРЕД всеми, выход ПО ПУТИ | both | spec: @fact:M-EPOCHS (новая эпоха — новый путь); code: `xtask/src/codegen/layout.rs:92-98` |
| 933 | [Ф4] Р2: Политику несёт СХЕМА, молчание схемы — ошибка генерации | code | `xtask/src/codegen/open_vocabulary.rs:9-16` (+ те же формулы в докблоках `empty_policy.rs`, `optional_shapes.rs`, `strictness.rs`) |
| 947 | [Ф4] Р3: Доменные типы СОХРАНЯЮТСЯ, эмит по `x-rust-type` | both | spec: #placement @fact:MIGRATION-TAXONOMY («Runtime compatibility shims are forbidden»); code: `xtask/src/codegen/domain_types.rs:13-36` |
| 959 | [Ф4] Р4: `Event` журнала становится СГЕНЕРИРОВАННЫМ, плечи `match` тем же шагом | both | spec: #agents @fact:AGENT-GATES G9; code: `schemas/journal/e1/journal.jtd.json:28-30` + `crates/vibe-wire/src/generated/journal/e1/journal/mod.rs:44-61` |
| 972 | [Ф4] Р5: Преобразований у генератора СЕМЬ, а не четыре | code | `xtask/src/codegen/postproc.rs:1-17` (перечень пассов; сегодня их восемь — дерево переросло число рулинга) |
| 990 | [Ф4] Р6: `codegen.rs` раскалывается в каталог-модуль ПЕРВЫМ делом | code | `xtask/src/codegen/layout.rs:1-8` (раскол по шву ответственности, 586/600) |
| 996 | [Ф4] Р7: Ф4.3 садится РЕХЕТОМ (К4), «реестр + маркер» — дефер | code | `tools/self-check.sh:235-256` (рехет wire-derive против baseline) |
| 1012 | [Ф4] Р8: Словарь в ОДНОМ словнике, подстановка ДО jtd-codegen | both | spec: #agents @fact:AGENT-GATES G9; code: `xtask/src/codegen/vocabulary.rs:1-30` |
| 1071 | [Ф4] Р9: Ф4.1 режется НАДВОЕ, порядок половин вынужденный | code | `xtask/src/codegen/vocabulary.rs:24-30` (отказ вместо паники бинаря) + `:38-44` (дом словника — `formats/`) |
| 1304 | Р10: Слой преобразований ключится ДОМОМ схемы | code | `xtask/src/codegen/mod.rs:157-178` (докблок `FormatOwner`, называет «Р10 плана») |
| 1321 | Р11: `timestamp` переводится на `"type": "timestamp"`, правка СХЕМЫ | code | `formats/vocabularies.json:41-47` + `xtask/src/codegen/domain_types.rs:45-47` |
| 1334 | Р12: Девятое преобразование `HashMap`→`BTreeMap`, безусловное | both | spec: #machinery @fact:M-CANONICAL-BYTES («One state — one byte sequence: sorted keys»); code: `xtask/src/codegen/ordered_maps.rs:8-33` |
| 1347 | Р13: У `x-default` два значения, отсутствие ключа — ошибка | code | `xtask/src/codegen/optional_shapes.rs:15-25` + `formats/vocabularies.json` (`"x-default": null/false` с описаниями «absent when…») |
| 1361 | Р14: Подсекции `Option<Box<T>>`→`Option<T>`; пустота — работа проектора | code | `xtask/src/codegen/optional_shapes.rs:26-29` + `xtask/src/codegen/empty_policy.rs:61-65` |
| 1376 | Р15: №4 остаётся правилом при нуле сайтов, обе мины закрыты явно | code | `xtask/src/codegen/strictness.rs:28-43` |
| 1395 | Р16: Сшивка с выходом по МНОЖЕСТВУ wire-значений, не по имени | code | `xtask/src/codegen/open_vocabulary.rs:18-28` |
| 1414 | Р17: `init_report.outcome_action` — ОТКРЫТЫЙ словарь | code | `schemas/init_report.jtd.json:53-62` (описание несёт решение и отвергнутую альтернативу) |
| 1426 | Р18: Сторож числа — enum-сайтов схемы == словарных enum'ов выхода | code | `xtask/src/codegen/open_vocabulary.rs:26-28` и `:82-90` |
| 1439 | Р19: Порядок пассов — закон; Р10 связывает ВЕСЬ слой | code | `xtask/src/codegen/postproc.rs:19-50` + `xtask/src/codegen/mod.rs:81-89` |
| 1454 | Р20: Rename снимается ТОЛЬКО тождественный | code | `xtask/src/codegen/snake_case.rs:10-36` |
| 1483 | Р21: Обязательность коллекции ОГРАНИЧИВАЕТ политику пустого | code | `xtask/src/codegen/empty_policy.rs:31-44` (+ `schemas/init_report.jtd.json:46-49`: «rule R21» в описании схемы) |
| 1636 | Р22: У `x-rust-type` два плеча, решает ФОРМА определения | code | `xtask/src/codegen/domain_types.rs:13-36` + `xtask/src/codegen/domain_types/rulings.rs:49-58` |
| 1673 | Р23: `x-rust-type` называет САМОДОСТАТОЧНЫЙ путь, пасс чинит импорты | code | `xtask/src/codegen/domain_types.rs:38-68` |

Цитаты спек-домов (spec/both), дословно:

- **394: Р1** — `vibevm/vibespecs/common/PROP-044-change-native-formats.xml`, секция `{#laws}`,
  `@fact:TERMS-SNAPSHOT-FROZEN-CHANNEL`: «**snapshot** ≡ `frozen = false` (the default:
  …) and **frozen** ≡ `frozen = true` (…) are **antonyms — the two states of the one
  `frozen` axis, with no third state.**» — это и есть причина, по которой рулинг отверг
  `Option<bool>`. Код-половина: `crates/vibe-core/src/manifest/package.rs:104-105` —
  `#[serde(default, skip_serializing_if = "is_false")]` / `pub frozen: bool`,` с доком
  «`[package].frozen` — the PROP-044 §2a immutability flag» (`is_false` — тот самый
  существующий помощник, :151); `formats/vocabularies.json:451-462` —
  `"yanked": { "type": "boolean", "metadata": { "x-default": false } }` и то же для
  `frozen`.
- **502: Р7** — PROP-044, `{#machinery}`, `@fact:M-EPOCHS`: «Light breaks (a new
  capability under must-understand) need no epoch at all.»; и `{#obligations}`,
  `@fact:THE-PUBLIC-SWITCH`: «break machinery reports instead of demanding (corpora
  regenerate freely, break notes are optional records)». Оба положения рулинга (эпоха не
  нужна; записка опциональна) несёт спека.
- **918: Р1** — PROP-044, `@fact:M-EPOCHS`: «A heavy break mints a new epoch at a new
  path (`/e4/…`)». Код-половина в «Рулинги, чей дом — КОД» не дублируется: цитата
  `layout.rs:95-98` приведена ниже в этом блоке: «Mirroring rather than flattening is
  contract, not taste: the schema's directory carries its epoch (PROP-044 §4.6 — a heavy
  break mints the new world as a new path), so `index/e1/entry` and a future
  `index/e2/entry` must stay distinct modules.»
- **947: Р3** — PROP-044, `{#placement}`, `@fact:MIGRATION-TAXONOMY`: «Runtime
  compatibility shims are forbidden; the only exception is frozen per-epoch *readers* of
  old manifests inside the indexer» — закон, которым рулинг отверг «`String` и конвертация
  на границе». Код-половина: `xtask/src/codegen/domain_types.rs:13-16`: «the RUST TYPE a
  definition binds to is a policy the schema declares, never a guess».
- **959: Р4** — PROP-044, `{#agents}`, `@fact:AGENT-GATES`: «G9: a vocabulary exists in
  exactly one schema; both wire sides, Rust types, docs and prose lists are generated from
  it (L1+L3).» Код-половина: `schemas/journal/e1/journal.jtd.json:28-30` —
  `"x-rust-type": "Event"` + `"discriminator": "kind"`, и сгенерированный
  `crates/vibe-wire/src/generated/journal/e1/journal/mod.rs:44-61`: `pub enum Event` c
  плечами-ньютайпами `EntrySetReplaced(Box<EventEntrySetReplaced>)`, `Initialised(Box<EventInitialised>)`.
- **1012: Р8** — PROP-044, `@fact:AGENT-GATES`, G9 (тот же якорь). Код-половина:
  `xtask/src/codegen/vocabulary.rs:3-9` — цитата в разделе КОД ниже.
- **1334: Р12** — PROP-044, `{#machinery}`, `@fact:M-CANONICAL-BYTES`: «One state — one
  byte sequence: sorted keys, injected clocks (a writer never calls `now()`; time arrives
  as input), pinned encodings, deterministic compression.» Код-половина:
  `xtask/src/codegen/ordered_maps.rs:10-12` — цитата в разделе КОД ниже.

## Рулинги без дома — по одному подразделу на каждый

### 473: Р4 — Одиннадцать литералов правятся поимённо

**План говорит** (дословно, суть):
> `VersionEntry` не выводит `Default`, среди его полей есть типы без `Default`, а
> `..Default::default()` не встречается во всём крейте ни разу: одиннадцать литералов
> перечисляют все ~30 полей поимённо, и новые поля дописываются во все одиннадцать …
> *Отвергнуто:* вывести `Default` ради `..Default::default()` — недоступно по типам и
> меняет больше, чем экономит.

**Что искалось и не нашлось** (паттерн + счётчик + контроль):
- Grep `поимённо|литерал|одиннадцат` по `spec/` → 2 попадания, оба нерелевантны
  (`vibevm/vibespecs/research/schema-evolution-2026-08/01-measure-our-wire-format-claudez.xml:57` —
  про rename-варианты `NamingConvention`; `12-HANDOFF.xml:221` — про пустые файлы
  харнесса). Ни одно не про Default литералов записи.
- Grep `impl Default for VersionEntry` по `crates/` → 0 (у записи Default нет и сегодня;
  8 `impl Default for` в `crates/vibe-wire/src/behaviour/` — все у подсекций и
  `NamingConvention`).
- Комментария/докблока, объясняющего ПОЧЕМУ у записи нет Default и почему литералы
  перечисляют поля поимённо, нет нигде: `records.rs` (билдер `minimal`) объясняет
  фикстурность и часы-вход, `projections.rs:7-12` — другую историю (Default подсекций
  рукописный, ибо кодген стёр бы derive).
- **Контроль непустоты (§0.7):** те же инструменты на заведомо существующих лексемах
  ненулевы: Grep `impl Default for` по `crates/vibe-wire/src/` → **8** находок; Grep
  `Default::default` по `crates/vibe-index/src/` → **8 строк** (`index/primary.rs:161-169`).
  Ноль по смыслу рулинга — не сломанный греп.

**Что будет потеряно при свёртке** (одно предложение):
Потеряется граница, которую ничто в дереве не сторожит, — «у самой `VersionEntry` Default
не выводится, поля перечисляются поимённо» (при том, что `Default::default()` у подсекций
уже появился в `primary.rs:161-169`): будущая сессия, добавляя поле, может «упростить»
выводом Default у записи и молча сменить семантику построения записи из фикстур.

## Рулинги, чей дом — КОД, а не спека

- **424: Р2** → `crates/vibe-index/src/index/memory.rs:334-337` → «Versions whose
  `must_understand` names a capability this build lacks are detected here — above the
  parsers, which stay pure bytes→types — and ENTER `by_pkgref` like every other version,
  with a WARN and a record in `quarantined`.»; имя набора — `crates/vibe-index/src/index/quarantine.rs:26-27`
  → «NOT the same vocabulary as a package's `provides.capabilities` — these are
  capabilities of the READER (PROP-044 §4.5), not of the package.»
- **445: Р3** → `crates/vibe-index/src/main.rs:58-66` → «Install the tracing subscriber
  unconditionally — a binary's job, not the library's. One lever, `VIBE_LOG` (default
  `warn`); there is no `RUST_LOG` fallback and no second lever — the global `--log-level`
  flag is not one either: it folds INTO `VIBE_LOG` … WARN-level observability (quarantine
  refusals on load, auto-commit-push outcomes) must be on for every subcommand, not only
  the flag-gated ones.»
- **480: Р5** → `crates/vibe-index/src/cli/add.rs:183-185` → «PROP-044 §2a — the
  manifest's `frozen` reaches the catalog entry through the `add` projection path (one of
  the two disjoint paths an entry is born from; the other is the org scanner).»; зеркально
  `crates/vibe-index/src/scanner/org_walk.rs:295-298` → «…through the org-scanner
  projection path (the second of the two disjoint birth paths; `cli::add` covers the
  first).»; сами проекции — `add.rs:130` и `org_walk.rs:241` (`frozen: pkg.frozen`).
- **493: Р6** → `crates/vibe-index/docs/format.md:72` → «## `by-name/<name>.json` —
  candidate set for one bare name» (исправленный путь); `:132` — поле `"group"` в примере
  записи; `:177-178` → «**absent from the JSON**, not written as `null`. `homepage`,
  `describes`, `resolved_commit`, `license`, `workspace_origin` and …» (исправленное
  утверждение о null).
- **933: Р2** → `xtask/src/codegen/open_vocabulary.rs:9-16` → «Which enums open is
  decided per vocabulary by the schema's `metadata."x-vocabulary": "open"` / `"closed"`
  annotation and by nothing else: measured, an enum that must open (`PackageKind`) and one
  that must not (`NamingConvention`) come out of the generator syntactically
  indistinguishable, so the pass takes its decision from the schema side of the stitch —
  and a missing annotation is a generation error, not a default, because the one thing
  this pass may not do is guess.» (та же формула — «A missing annotation … is a generation
  error, not a default» — в `empty_policy.rs:19-21` и `optional_shapes.rs:20-21`).
- **972: Р5** → `xtask/src/codegen/postproc.rs:4-17` → «This file is the driver plus the
  first pass; the other seven — renaming field identifiers to snake_case (dropping the
  identity renames), turning wire maps into ordered `BTreeMap`s, collapsing optional
  collections per the schema's `x-empty`, lifting the `Box` off optional scalars and
  structures per the schema's `x-default`, stamping `#[serde(deny_unknown_fields)]` on the
  structs of formats the registry marks `foreign_parsers = "none"`, binding the domain
  Rust types the schema's `x-rust-type` names …, and opening vocabularies per the schema's
  `x-vocabulary` — live in the sibling … modules, split along those responsibility seams
  as the set outgrew the 600-line budget.»
- **990: Р6** → `xtask/src/codegen/layout.rs:1-8` → «Split from `mod.rs` (the driver:
  binary lookup, home routing, emission, the drift check) along the responsibility seam —
  every test the old single file carried exercises exactly this half — and because
  `mod.rs` sat at 586 of its 600-line budget with a post-processing layer still to be
  wired into `generate_into`.»
- **996: Р7** → `tools/self-check.sh:238-245` → «A flat ban is impossible today:
  handwritten Serialize/Deserialize derives stand in 139 files across eleven crate keys
  (measured 2026-08-17, frozen below), and almost all of it is lawful — … A
  named-exception list of that length rots within a week, so the form is a RATCHET:
  today's count is frozen per crate in wire-derive-baseline.json and any GROWTH goes
  red.»
- **1071: Р9** → `xtask/src/codegen/vocabulary.rs:24-30` → «The same pass refuses, with a
  recipe, every input that would otherwise reach jtd-codegen as a panic: a `{"ref": "x"}`
  with no matching definition dies inside the binary with `no entry found for key`, naming
  neither the schema nor the name — and a dependency chain that leaves the home, or loops
  back on itself, is refused with the route it took.»; дом словника — `:38-42` →
  «`formats/` is the house of data about formats …; the schema scanner collects
  `*.jtd.json` under the schema homes only, so a plain `.json` here is vocabulary data,
  never a schema the generator would try to build as a format of its own.»
- **1304: Р10** → `xtask/src/codegen/mod.rs:157-169` → «Whose formats a schema home holds
  — the question Р10 of the change-native plan answers, and the reason the transformation
  layer is not applied everywhere the generator runs. … `Foreign` is a vendored package's
  schema home — its output is that package's public Rust API, and our wire policy has no
  standing to bind it to our release train. Withholding the passes there costs nothing
  today: the engine's schema carries no discriminator union, so the boxing pass was
  already a no-op on it, which is checkable by regenerating and comparing bytes.»
- **1321: Р11** → `formats/vocabularies.json:41-47` → `"timestamp": { "metadata": {
  "x-rust-type": "chrono::DateTime<chrono::Utc>", "description": "RFC 3339 timestamp —
  `chrono::DateTime<Utc>` in code." }, "type": "timestamp" }` — правка схемы посажена;
  последствие (осиротевшие импорты `DateTime`/`FixedOffset`) несёт
  `xtask/src/codegen/domain_types.rs:45-47` → «a substitution may orphan an import item —
  the alias line was the last place the generator's own `DateTime` / `FixedOffset` tokens
  stood — and the pass removes EXACTLY those items, no more.»
- **1347: Р13** → `xtask/src/codegen/optional_shapes.rs:15-21` → «An optional SCALAR is
  decided by `metadata."x-default"`: `null` keeps the `Option` (an absent key means "no
  value"), a boolean literal collapses the field to the bare `bool` (PROP-044 §2b's
  two-part boolean axis …), and a missing key is a generation error — the policy lives on
  the schema side and is not derivable from the generated Rust.»; пример «отсутствие —
  факт» — `formats/vocabularies.json:359-364` → «SPDX expression; absent when the package
  declares none.» + `"x-default": null`.
- **1361: Р14** → `xtask/src/codegen/optional_shapes.rs:26-29` → «An optional STRUCTURE
  needs no annotation at all and does not read one: the `Option` stays because accepting
  `{}` from a foreign writer is the type's job, while normalising an empty object is
  projector work, not a skip predicate's.»; вторая половина —
  `xtask/src/codegen/empty_policy.rs:61-65` → «the collapse removes the state
  `Some(empty)`, which the projector never produced (empty normalisation is projector
  work, contract annex B.2)».
- **1376: Р15** → `xtask/src/codegen/strictness.rs:31-43` → «two records claiming one
  schema must agree on the role (strictness is a property of the format, and a schema
  feeding `none` and `many` at once has no single policy — a loud refusal naming both
  records, never "first one wins" …), and a record naming a schema no phase has built yet
  (`handshake`'s plan-assigned path) is skipped BY NAME with a line in the run output —
  the schema scanner never sees the file, so silence there would read as checked when
  nothing was.»
- **1395: Р16** → `xtask/src/codegen/open_vocabulary.rs:18-22` → «the schema side
  collects every enum site (an object with an `"enum"` array of strings) keyed by its set
  of wire values — not by name, for the generator sorts the variants and mints the type
  name on its own, while both sides carry the value set verbatim.»
- **1414: Р17** → `schemas/init_report.jtd.json:56` → «Open vocabulary: the register of
  outcomes is a growing account of what the command did to a file and may gain values
  (`replaced`, `skipped`); an unfamiliar outcome has an obviously safe behaviour — show
  the string as it came, in `Unknown`; and nothing builds a path or a decision from the
  value. Closed was rejected: a report written by a newer `vibe` would then break an older
  reader's parse outright, for no benefit.»
- **1426: Р18** → `xtask/src/codegen/open_vocabulary.rs:26-28` → «After the file, the
  count of vocabulary enums found must meet the schema's site count exactly — the tally
  that keeps THE UNION SKIP RULE honest rather than silent.»; и `:84-86` → «plus the
  NUMBER OF SITES (not of distinct sets) — the tally the Rust-side scanner must meet
  exactly, the tripwire that keeps a silently skipped vocabulary from passing for
  processed.»
- **1439: Р19** → `xtask/src/codegen/postproc.rs:19-23` → «The passes run in a fixed
  ORDER, and the order is a rule, not a taste: a pass keyed to the generator's emission
  shape must run while the file is STILL that emission. Boxing is keyed to the shape (…),
  so it runs first. … Opening vocabularies then writes hand-rolled `impl Serialize` /
  `impl Deserialize` blocks into the file — text the pinned emission shape does not
  contain — and any shape-keyed pass running after it would be reading a document that is
  no longer the generator's.»; «третий дом обязан назвать владельца» —
  `xtask/src/codegen/mod.rs:86-89` → «it is stated here rather than re-derived
  downstream, so a home added tomorrow has to name its owner instead of defaulting to one
  silently.»
- **1454: Р20** → `xtask/src/codegen/snake_case.rs:12-20` → «the pass renames every
  struct field identifier to its snake_case form, then removes the `#[serde(rename = …)]`
  that would merely repeat the new identifier's identity. Only that one: a rename whose
  wire string DIFFERS from `snake_case(identifier)` carries information — the schema
  declared a camelCase property ON THE WIRE — and it stays. The invariant …: **the wire
  does not move in either branch.** … There is no third branch.»; живой случай — `:27-34`
  → «`registry_sync_report` declares a property named `ref`, a Rust keyword … A schema
  property that is a Rust keyword is a permanent class, not an accident of this tree».
- **1483: Р21** → `xtask/src/codegen/empty_policy.rs:31-38` → «REQUIREDNESS BOUNDS THE
  POLICY (rule R21, owner ruling P21): a JTD `properties` member is required, so a writer
  omitting an empty one would produce a document invalid by its own schema — the exact
  shape of a wrong answer that looks right. Required + `omit` is therefore a generation
  error of its own …, and the only lawful policy for a required collection is `emit`»;
  подтверждение в самой схеме — `schemas/init_report.jtd.json:48` → «Required member: an
  idempotent init that touched nothing writes `[]`, never an absent key — omitting a
  required collection would produce a document invalid by this same schema (rule R21).»
- **1636: Р22** → `xtask/src/codegen/domain_types.rs:16-27` → «`metadata."x-rust-type"`
  names one of the two halves of the emitted declaration, and the definition's own JTD
  form decides which half — so the reading cannot pick the wrong one. A `type` (primitive)
  form is emitted as an alias, and the annotation is its RIGHT SIDE: … An object, enum or
  discriminator form is emitted as a named type, and the annotation is its NAME»; типы
  плеча — `xtask/src/codegen/domain_types/rulings.rs:50-58` (`Arm::RightSide` /
  `Arm::Name`); исправленный дефект данных — `formats/vocabularies.json:29` →
  `"x-rust-type": "vibe_core::Group"`.
- **1673: Р23** → `xtask/src/codegen/domain_types.rs:38-45` → «P23 — the rule this pass
  carries, paid for in full right here: an `x-rust-type` annotation names a path
  resolvable WITHOUT the file's own imports (`vibe_core::Group`, `semver::Version`,
  `chrono::DateTime<chrono::Utc>`, never a bare `Group` or `Utc` that resolves only while
  the generator's import happens to be in scope — an annotation whose resolvability rides
  on the generator's import breaks the moment the emission shape shifts, the exact
  coupling this layer exists to break).»; формы вывода под панелью — `:58-68` → «A braced
  import down to its last survivor is written unbraced (`use a::b;`), because rustfmt
  rewrites `use a::{b};` … And a line removed from between two blanks takes the second
  blank with it …».

## Контроли

1. **Класс `spec` предъявлен** (§3.1): 502: Р7 — обе половины рулинга несут дословные
   фразы PROP-044: `@fact:M-EPOCHS` «Light breaks (a new capability under must-understand)
   need no epoch at all.» (строка 276 спеки) и `@fact:THE-PUBLIC-SWITCH` «break machinery
   reports instead of demanding (corpora regenerate freely, break notes are optional
   records)» (строки 415-417 спеки). Цитаты — в «Карте домов» выше.
2. **Класс `code` предъявлен** (§3.2): 22 рулинга; образцовый — 1673: Р23, докблок
   `xtask/src/codegen/domain_types.rs:38-45` прямо назван «P23 — the rule this pass
   carries, paid for in full right here» и несёт решение, причину и отвергнутые
   альтернативы. Все 22 цитаты — в разделе «Рулинги, чей дом — КОД».
3. **Каждый `none` — с контролем непустоты** (§3.3, §0.7): единственный `none` — 473: Р4;
   контроли при нём: Grep `impl Default for` по `crates/vibe-wire/src/` → 8 находок;
   Grep `Default::default` по `crates/vibe-index/src/` → 8 строк; Grep
   `поимённо|литерал|одиннадцат` по `spec/` → 2 нерелевантных попадания (research-корпус),
   по `crates/`, `xtask/` — 0. Инструмент ненулевой на заведомо существующем; ноль по
   смыслу рулинга — находка, не поломка.
4. Пасс не моноваленён: сегмент дал 1 `spec`, 6 `both`, 22 `code`, 1 `none` — все четыре
   класса предъявлены (§3: «сегмент почти наверняка содержит рулинги всех четырёх
   классов» — подтвердилось).

## Счёт

Рулингов в сегменте (строки 239–1720, по заголовкам `^\*\*Р<число>\.`): **30**.
Из них: **spec — 1**, **both — 6**, **code — 22**, **none — 1**. Пропущенных нет: строк
в «Карте домов» — 30. Блоки: Ф1.4 (394–502) — 7; нарезка Ф4 (918–1071) — 9; Р10–Р15
(1304–1376) — 6; Р16–Р21 (1395–1483) — 6; Р22–Р23 (1636–1673) — 2. Ж1–Ж8 фазы 3 — не
«Р»-рулинги и в карту не входят. Весь файл содержит 68 `^\*\*Р<число>\.` заголовков (в
§0.9 пакета названо 69; после Р35 нумерация прыгает на Р38 — см. Deviations отчёта).
