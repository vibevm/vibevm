# Смертность плана — карта домов, сегмент D (ТЗ, строки 1721–3001)

## Метод

Сегмент (строки 1721–3001 `TZ-CHANGE-NATIVE-FORMATS-v0.1.md`) прочитан целиком;
рулинги перечислены грепом заголовков — строгим `^\*\*Р[0-9]+\.` и поверх него
рыхлым `^\s*\*\*Р[0-9]`, потому что у Р36 (строка 2145, отступ) и Р37 (строка
2282) после номера стоит тире, а не точка, и строгий шаблон их роняет; в
сегменте 40 рулингов. Дом искался в порядке карты §2: сперва PROP-005 (прочитан
целиком — он выправлен против дерева 2026-08-18) и PROP-044 §2–§9, затем
прицельные грепы по `spec/**`, `schemas/**`, `formats/**`,
`xtask/src/codegen/**`, `crates/vibe-wire/**`, `crates/vibe-index/**`,
`crates/vibe-registry/src/index_client/**`, `tools/self-check.sh`; `BACKLOG.md`
читался только как жильё деферов. Цитатой считалась фраза, несущая и утверждение,
и его причину; якорь спеки даётся как `файл §секция #ЯКОРЬ-ФАКТА`, код — как
`file:line`. Всякий ноль, читаемый как «дома нет», снабжён контролем непустоты
(§0.7): тем же инструментом найдено слово, которое он обязан найти, со счётчиком.

## Коммиты посадки по фазам

| фаза / подшаг | пометка в плане (строка) | коммиты |
|---|---|---|
| Ф4.2c-1 — трейт-этаж (Р24) | 1964: «СЕЛА 2026-08-17» | `95feb37f` |
| Ф4.2c-2 — имя варианта (Р27) | 1996: «СЕЛА 2026-08-17» | `dca804db` |
| Ф4.2c-3a — общий модуль (Р28–Р30) | 2020–2021: «СЕЛА 2026-08-17» | `37496cab` |
| Ф4.2c-3b — реэкспорт, поведение, ребро, `--kind` (Р25+Р26+Р31+Р33–Р35) | 2062–2063: «СЕЛА 2026-08-17» | `53f8c429` (42 файла, +1051/−1054) |
| Ф4.2c-4 — три ридера без ридера (G11, Р36) | 2114–2115: «СЕЛА 2026-08-17 — ФАЗА Ф4.2c ЗАКРЫТА, четыре шага из четырёх» | `b7464ea0` |
| Ф4.3 — запрет рукописного провода | 2188: «СЕЛА 2026-08-17 — ФАЗА Ф4.3 ЗАКРЫТА» | `ee4f7230` (52-й шаг панели, база 139) |
| Ф5.1 — золотые корпуса | 2242: «СЕЛА 2026-08-17» | `29043890` |
| Ф5.3 — окно перелома, `EPOCHS.toml` | собственного хэша текст не называет; порядок по Р37 назван строкой 2295: «Ф5.1 → Ф5.3 → Ф5.2» | — (вошёл в закрытие фазы 5) |
| Ф5.2 — `wire-diff` | 2294: «СЕЛА 2026-08-17 — ФАЗА 5 ЗАКРЫТА, три шага из трёх» | `ecd2e955` |
| Ф6.1a — поверхность писателя в один дом (Р38) | 2548: «СЕЛА 2026-08-17» | `c6a984e9` |
| Ф6.1b-0 — пасс учится форме даты (Р47) | 2556: «СЕЛА 2026-08-17» | `e1d77d5b` |
| Ф6.1b — схема вечного файла и оракул | 2560: «СЕЛА 2026-08-17» | `2668fa55` |
| Ф6.1c — индекс публикует хэндшейк | 2570: «СЕЛА 2026-08-17» | `69bdad89` |
| Ф6.1d — клиент спрашивает хэндшейк первым | 2586–2587: «СЕЛА 2026-08-17 — Ф6.1 ЗАКРЫТА, пять шагов из пяти» | `5c023848` |
| Ф6.2a — загрузчик не выбрасывает (Р52) | 2919–2920: «⇒ Ф6.2 ЗАКРЫТА 2026-08-17: четыре шага из четырёх» | `5fabcea6` |
| Ф6.2b — семь глаголов CLI отвечают `unavailable` | 2920 | `fa50b653` |
| Ф6.2c — шесть отвечающих поверхностей HTTP (Р49, Р55) | 2920 | `0798614f` |
| Ф6.2d — `--log-level` (Р51, Р53) | 2920 | `ce3de248` |
| решения Р52–Р55 (docs) + замер Ф6.2c | 2921 | `5f308eb0`, `a02ccf82`, `72702642`; замер `d7aff0ce` |

## Карта домов — по одной строке на рулинг

| строка | рулинг (первые ~8 слов) | класс | дом (якорь спеки / file:line) |
|---|---|---|---|
| 1729 | Р24 Трейт-этаж эмитится БЕЗУСЛОВНО, и `Default` | code | `xtask/src/codegen/derive_floor.rs:1-47` — докблок пасса несёт всё три «почему» (безусловность, отсутствие `Default`, законность `Eq` с триггером) |
| 1765 | Р25 `ValueEnum` не переезжает, и `--kind` | code | `crates/vibe-index/src/cli/list.rs:25-30` + `cli/kinds.rs:21-35` |
| 1788 | Р26 Методы и трейт-`impl`'ы переезжают В `vibe-wire` | code | `crates/vibe-wire/src/lib.rs:23-30` — «a type this crate defines can carry its behaviour … ONLY here — the orphan rule bars every other home» |
| 1805 | Р27 Имя ВАРИАНТА объявляется схемой, картой по WIRE-ЗНАЧЕНИЮ | code | `formats/vocabularies.json:16-25` + `xtask/src/codegen/domain_types/variants.rs:1-25` |
| 1846 | Р28 Дедупликация выхода ложится ПЕРЕД реэкспортом | both | spec: PROP-005 §2.6 `#ENTRY-SCHEMA` («defined once, in the shared `version_entry` vocabulary … the schema language has no cross-file reference») + §2.12 `#RUST-TYPES`; code: `xtask/src/codegen/shared_module.rs:5-17` («cannot even be *expressed* while one name denotes several types, which is why this step precedes it») |
| 1868 | Р29 Общий модуль — не десятый пасс, а третья фаза | code | `xtask/src/codegen/shared_module.rs:19-35` (три фазы прогона) + `crates/vibe-wire/src/lib.rs:32-43` |
| 1880 | Р30 Эпоха приезжает в ИМЯ фрагмента | none | подраздел ниже |
| 1892 | Р31 Теряемые трейты разделены измерением на три класса | none | подраздел ниже |
| 1908 | Р32 `Repomd`/`RepomdFileEntry` выходят из периметра | spec | PROP-005 §2.12 `#TWO-SHAPES-STAY-HAND-WRITTEN-AND-SAY-WHY`: «its `size` is a `u64` where the schema language reaches only `u32` (an open owner fork, `BACKLOG.md` B-056)»; триггер живёт в BACKLOG B-056 (второй, живой) |
| 1921 | Р33 Слой поведения живёт в `vibe-wire/src/behaviour/` | code | `crates/vibe-wire/src/behaviour/mod.rs:1-17` + `lib.rs:92-96` |
| 1935 | Р34 Имя корневого типа ОБЪЯВЛЯЕТСЯ аннотацией | code | `schemas/index/e1/by_name.jtd.json:3-4` (`"x-rust-type": "NameEntry"`, «Root type `NameEntry`») + `xtask/src/codegen/domain_types/rulings.rs:106-108` (корень читается, ключ `(the root)`); закон «DECLARED rather than minted» — `schemas/hello/e1/hello.jtd.json:3` |
| 1949 | Р35 `BindingSite` входит в шаг | spec | PROP-005 §2.12 `#RUST-TYPES`: «`BindingSite` from `by_purl`» — сведение словарей в сгенерированный дом |
| 2145 | Р36 ридер читает в СГЕНЕРИРОВАННЫЙ тип, писатель рукописный | code | `crates/vibe-index/tests/round_trip_published.rs:8-15` + `tests/wire_parity_inverted.rs:9-16` |
| 2282 | Р37 порядок остатка фазы 5 переставлен: Ф5.3 перед Ф5.2 | both | spec: PROP-044 §4.7 `#M-BREAK-WINDOW` + §7 `#THE-PUBLIC-SWITCH`; code: `formats/EPOCHS.toml:23-26` («wire-diff, whose entire behaviour these two flags decide»), `tools/self-check.sh:30-35` |
| 2356 | Р38 Поверхность писателя — нормативное значение | both | spec: PROP-005 §2.4 `#THE-WRITERS-OWN-SURFACE-IS-A-WHITELIST` («named once in the code so the two readers … cannot compare different sets»); code: `crates/vibe-index/src/index/mod.rs:46-61` (`WRITER_FILES`/`WRITER_DIRS`) |
| 2377 | Р39 `hello.json` живёт ВНЕ карты `repomd.files` | both | spec: PROP-005 §2.4 `#THE-HANDSHAKE-IS-NOT-AN-ENTRY-OF-THE-MANIFEST` + §2.13 `#REPOMD-LAST-LAW`; code: `crates/vibe-index/src/index/memory.rs:205-213` («above every world, so it follows even the manifest (Р39)»), `index/mod.rs:48-51` |
| 2402 | Р40 Хэндшейк — проекция РЕЕСТРА ФОРМАТОВ | code | `crates/vibe-index/src/index/memory.rs:401-408`: «A projection of the FORMAT REGISTRY, not of the journal (Р40): both numbers come from the generated `FormatId` … no clock enters» |
| 2414 | Р41 Хэндшейк ищется ПЕРВЫМ | both | spec: PROP-005 §2.1 `#WHY-THE-HANDSHAKE-IS-ASKED-BEFORE-THE-MANIFEST` («readable exactly when the old address no longer serves a catalog»); code: `crates/vibe-registry/src/index_client/mod.rs:126-145` |
| 2431 | Р42 `successor` ЧИТАЕТСЯ и НАЗЫВАЕТСЯ, но не следуется | both | spec: PROP-005 §2.1 `#A-SUCCESSOR-IS-NAMED-NEVER-FOLLOWED`; code: `schemas/hello/e1/hello.jtd.json:38-43` («Read and named in the refusal, never followed automatically (Р42)»), `index_client/mod.rs:142-144` |
| 2442 | Р43 Набор ключей вечного файла ПОЛОН | both | spec: PROP-044 §3 `#ONE-ETERNAL-FILE` («Its keys never change meaning»); code: `schemas/hello/e1/hello.jtd.json:3` («The key set is complete from day one (Р43) … a schema describing less describes a different format») |
| 2455 | Р44 `min_client` — `semver::Version`; `sunset` — `timestamp` | code | `schemas/hello/e1/hello.jtd.json:24-30` («the tolerance … covers unknown KEYS, never a lie about a known key's type»), `:65-88` (`world_sunset`: «a bare string would give a date a SECOND legal spelling in the system») |
| 2467 | Р45 «Своя эпоха клиента» НЕ заводит нового числа | code | `schemas/hello/e1/hello.jtd.json:52-56` («the CLIENT does not mint a constant of its own, it reads its epoch from `formats/REGISTRY.toml` through the generated `FormatId::epoch()`»), `index_client/mod.rs:135-138`, `memory.rs:410-415`, `crates/vibe-wire/src/generated/format_id/mod.rs:107-131` |
| 2480 | Р46 Отказ хэндшейка — вычисляемое содержание | both | spec: PROP-005 §2.1 `#A-PROBE-HAS-THREE-OUTCOMES-NOT-TWO` («carrying the offered epochs, this build's epoch, a recipe, and whatever the document said in `min_client` / `notice` / `successor`»); code: `index_client/mod.rs:107-123` (`ProbeOutcome::Refused`) |
| 2492 | Р47 Опциональная дата вынудила шаг ПЕРЕД схемой | code | `xtask/src/codegen/optional_shapes/emit.rs:295-308` («it had exactly one hole: JTD's `timestamp` renders as `DateTime<FixedOffset>`, and until a schema needed an OPTIONAL date no site ever asked») + `hello.jtd.json:82-88` |
| 2532 | Р48 Маркер `specmark::scope!` в `tests/` не ставится | none | подраздел ниже |
| 2616 | Р49 Карантин переезжает из пути ЗАГРУЗКИ в путь ОТВЕТА | both | spec: PROP-005 §2.19 `#QUARANTINE-IS-A-READERS-JUDGEMENT-AND-IS-NEVER-CARRIED` («the command line and the server agree **by construction**»); code: `crates/vibe-index/src/index/quarantine.rs:1-9` («Since the loader stopped dropping quarantined versions, this module is also THE single home of the answer path's judgement»), `memory.rs:199-203` |
| 2657 | Р50 Рецепт принадлежит ВОЗМОЖНОСТИ, а не формату | both | spec: PROP-005 §2.19 `#THE-RECIPE-HAS-ONE-HOME` («built in one place and never written as a literal at a call site … The per-capability table this grows into»); code: `quarantine.rs:20-28` (`UNDERSTOOD`, «grows as capabilities land») + `:105-117` (`recipe_for`) |
| 2683 | Р51 `--log-level` складывается с `VIBE_LOG` в ОДИН рычаг | both | spec: PROP-005 §2.11 `#THE-LOG-DIAL-AND-THE-VARIABLE-ARE-ONE-LEVER` («passing the flag SETS that variable … one thing with a coarse dial and a fine one»); code: `crates/vibe-index/src/main.rs:26-56` (`apply_log_level`, дословно «(Р51)», с отвергнутой альтернативой в `deviates`) |
| 2713 | Р52 Форма безопасного умолчания — ИМЕНОВАННЫЙ аксессор | both | spec: PROP-005 §2.19 `#THE-SAFE-DEFAULT-IS-A-CONSTRUCTION-NOT-AN-AGREEMENT` («asks the **named accessors** (`quarantine::usable_*`) … The asymmetry is stated in the doc-comments of both sides»); code: `quarantine.rs:5-9, 60-71` |
| 2768 | Р53 `--log-level` принимает ЗАКРЫТОЕ перечисление из шести | both | spec: PROP-005 §2.11 `#THE-LOG-DIAL…` («a closed set of six values»); code: `crates/vibe-index/src/cli/mod.rs:48-64` (`LogLevel`: «`off` is a member because `VIBE_LOG=off` is legal …») |
| 2794 | Р54.1 Дом ответа — тот же, что дом карантина | both | spec: PROP-005 §2.19 `#UNAVAILABLE-SHAPE` («carries the **full coordinate even where the envelope around it already names the package**»); code: `quarantine.rs:90-103` (`Unavailable`) + `recipe_for` |
| 2814 | Р54.2 Столкновение с рехетом Ф4.3 предсказано | code | `tools/self-check.sh:235-245` («the form is a RATCHET: today's count is frozen per crate in `wire-derive-baseline.json` and any GROWTH goes red»); неотinventory семи конвертов — дефер BACKLOG B-079 |
| 2834 | Р54.3 Живой дефект, который шаг закрывает по построению | none | подраздел ниже (утверждение переехало в код, истории нет) |
| 2845 | Р54.4 `dump --format jsonl` остаётся потоком ОДНОЙ формы | code | `crates/vibe-index/src/cli/dump.rs:44-50` («a line of any other shape in this stream is a break in the wire, and `dump` is bulk export, not an answer by NAME») |
| 2871 | Р55.1 Сырой `GET /v1/index/by-name/{name}` НЕ входит | spec | PROP-005 §2.19 `#WHICH-SURFACES-OWE-THE-ANSWER` («every surface that COMPUTES an answer owes the refusal; a surface that serves a stored file verbatim does not») + `#THE-RAW-FILE-WAS-NEVER-THE-ONE-KEEPING-SILENT` («hands back the record word for word, `must_understand` included — and that declaration IS the explanation of the refusal») |
| 2883 | Р55.2 Клиент в Ф6.2c не входит | none | подраздел ниже (дефер живёт в BACKLOG B-080) |
| 2892 | Р55.3 `unavailable` живёт в КОНВЕРТЕ, никогда в `VersionEntry` | both | spec: PROP-005 §2.19 `#A-REFUSED-VERSION-IS-A-404-CARRYING-ITS-REASON` («The judgement rides the envelope and never enters the record»); code: `quarantine.rs:51-58` (`is_usable`: «never a property of the record … never stored on the wire») |
| 2897 | Р55.4 Статус остаётся 404, говорит ТЕЛО | spec | PROP-005 §2.19 `#A-REFUSED-VERSION-IS-A-404-CARRYING-ITS-REASON` + §2.10 `#THE-BODY-CARRIES-NO-INSTANCE-MEMBER` («the refusal row, as an extension member, which is the mechanism that RFC provides for precisely this») |
| 2904 | Р55.5 Операционные счётчики остаются писательскими | both | spec: PROP-005 §2.19 `#THE-SAFE-DEFAULT…` («the mutations, and the operational counters ask the raw state deliberately»); code: `server/routes/admin.rs:25-27` и `server/routes/metrics.rs:11-13` («The counts are the WRITER's — everything the index HOLDS, including quarantined versions (R55.5) … do not "fix" them») |
| 2909 | Р55.6 Фикстура карантина — СВОЯ; `populated_state()` не трогается | none | подраздел ниже |

## Рулинги без дома — по одному подразделу на каждый

### 1880: Р30 — Эпоха приезжает в ИМЯ фрагмента

**План говорит** (дословно, суть):
> «Эпоха приезжает в ИМЯ фрагмента, а не в каталог общего дома. … фрагмент,
> изменившийся в e2, есть ДРУГОЙ фрагмент — у него другое имя, он ложится в тот
> же дом рядом, и миры разделены именем, а не каталогом. *Триггер
> пересмотра:* первая схема второй эпохи, тянущая фрагмент первой.»

**Что искалось и не нашлось** (паттерн + счётчик + контроль):
`epoch|Epoch` по `crates/vibe-wire/src/generated/shared/mod.rs` — 0; по
`formats/vocabularies.json` — 0 (тот же файл по `x-rust-type` даёт **5** —
инструмент жив); по `xtask/src/codegen/**` epoch-хиты есть только про каталоги
схем (`layout.rs:14,66,96` — «schema directory … carries its epoch»), не про
имя фрагмента в общем доме. Правило перспективное: вторая эпоха ещё не минтована,
общий дом эпоху не знает.

**Что будет потеряно при свёртке** (одно предложение):
Решение, что изменившийся в новой эпохе фрагмент кладётся в общий дом ПОД НОВЫМ
ИМЕНЕМ, а не в эпохальный подкаталог — и его триггер (первая e2-схема, тянущая
e1-фрагмент) — будет некому вспомнить в день, когда триггер сработает.

### 1892: Р31 — Теряемые трейты разделены измерением на три класса

**План говорит** (дословно, суть):
> «`Ord` / `Hash` / `PartialOrd` — **0 сайтов**: не нужны никому. Комментарий
> `crates/vibe-index/src/scanner/manifest.rs:14`, называющий `Ord` причиной
> дублирования словаря, лжёт с момента написания и правится тем же шагом… ·
> `Copy` — **19 сайтов** у `PackageKind`… · `Default` — **15 сайтов**,
> восстанавливается рукописным `impl` рядом с `generated/`.»

**Что искалось и не нашлось** (паттерн + счётчик + контроль):
рассуждение о трёх классах теряемых трейтов и числах сайтов — `Ord|PartialOrd`
по `spec/**` (только чужие контексты), по `crates/vibe-wire/src/behaviour/**`
(докблоки словарей/проекций молчат о трейтах) — 0 содержательных; контроль:
`PackageKind` по `crates/vibe-index/src` — **122 вхождения в 26 файлах**.
Ложный комментарий действительно исправлен (новая шапка
`scanner/manifest.rs:1-20` объясняет закрытую/открытую конверсию), но сама
классификация потерь нигде не записана.

**Что будет потеряно при свёртке** (одно предложение):
Измеренное обоснование, почему реэкспорт стоил именно `.clone()` на 19 сайтах и
почему `Ord`/`Hash` никому не были нужны, — то есть цена, уже заплаченная и
забытая, и защита от повторного «оптимизирующего» возврата дубликата ради
`Copy`.

### 2532: Р48 — Маркер `specmark::scope!` в `tests/` не ставится

**План говорит** (дословно, суть):
> «правило „новый файл несёт `scope!`“ связывает `src/` продуктовых и движковых
> крейтов… а интеграционные тесты этого крейта живут без него единообразно…
> *Следствие для будущих пакетов:* клауза о `scope!` пишется с оговоркой „если
> соседи по КАТАЛОГУ его несут — проверь прогоном, а не допущением“.»

**Что искалось и не нашлось** (паттерн + счётчик + контроль):
`specmark::scope` по `spec/**` — хиты только чужие (счётные таблицы кампаний в
`vibevm/vibespecs/terraforms/**`, `vibevm/vibespecs/design/**`, PROP-037 про vibe-cli), самого правила
про `tests/` нет; контроль того же грепа по коду: `specmark::scope!` в
`crates/vibe-index/src` — **70 файлов**, в `crates/vibe-index/tests` — **0**
(единообразие подтверждено, но записано только фактом дерева, не правилом).

**Что будет потеряно при свёртке** (одно предложение):
Применение закона единообразия к интеграционным тестам и рецепт оговорки для
будущих пакетов («проверь прогоном, а не допущением») — очередной пакет снова
потребует маркер с потолка и снова снимет его двадцатым исключением.

### 2834: Р54.3 — Живой дефект, который шаг закрывает по построению

**План говорит** (дословно, суть):
> «`crates/vibe-index/src/cli/get.rs` в ветке пустого списка версий печатает
> `args.version.unwrap()`… Ветка достижима **без** `--version`… и `get`
> **ПАНИКУЕТ**. … Дефект старше Ф6.2a… Ф6.2b удаляет его не заплатой, а тем,
> что ровно эта ветка и становится ответом `unavailable`.»

**Что искалось и не нашлось** (паттерн + счётчик + контроль):
история дефекта (`unwrap`, паника, «старше Ф6.2a») — 0 в `spec/**` и в докблоках
`cli/get.rs`; контроль: `unavailable` по `crates/vibe-index/src/cli/get.rs` —
**15 вхождений** (ветка ныне отвечает `unavailable`/`found:false`, паники нет —
`cli/get.rs:85-126` с комментариями семантики). Утверждение переехало,
обоснование (история дефекта) — нет.

**Что будет потеряно при свёртке** (одно предложение):
Знание, что пустой список версий когда-то был паникой в доменной логике и закрыт
именно конструкцией ответа, — то есть прецедент «не заплаты, а сменой
ветки», на который ссылался стиль фазы.

### 2883: Р55.2 — Клиент в Ф6.2c не входит

**План говорит** (дословно, суть):
> «`index_client` не читает `must_understand` нигде, а „latest“ ВЫВОДИТ сам из
> полного нефильтрованного списка… резолвер способен выбрать версию, которой
> сборка пользоваться не может… Решение крупнее шага и хвостом ехать не должно.
> Записано как `BACKLOG.md` **B-080** с триггером.»

**Что искалось и не нашлось** (паттерн + счётчик + контроль):
в `spec/**` и коде рассуждения нет — `must_understand` по
`crates/vibe-registry/src` — **0** при контроле тем же грепом по
`crates/vibe-index/src` — **32 в 10 файлах**. Сам дефер при этом уже имеет
полноценный постоянный дом вне классов §1: `BACKLOG.md` B-080 (P2, open,
`@fact:B080-*` — факт, место неверного ответа, цена починки, триггер; запись
прямо называет решение Р55.2).

**Что будет потеряно при свёртке** (одно предложение):
По существу — ничего: содержание живёт в B-080 целиком; теряется только
привязка дефера к шагу Ф6.2c и мотив «решение крупнее шага» в контексте фазы.

### 2909: Р55.6 — Фикстура карантина — СВОЯ; `populated_state()` не трогается

**План говорит** (дословно, суть):
> «Замер перечислил семь серверных тестов, которые покраснели бы от правки
> общей фикстуры… Краснота от смены ФИКСТУРЫ не доказывает ничего и стоит
> перенацеливания семи стражей… **красный существующий серверный тест есть
> дефект правки, а не повод править тест.**»

**Что искалось и не нашлось** (паттерн + счётчик + контроль):
правило «красный существующий тест — дефект правки, а не повод править тест» —
0 в `spec/**` (PROP-044 §8 о гейтах его не содержит) и в докблоках тестов;
контроль: `populated_state` по `crates/vibe-index/tests` — **25 вхождений в 2
файлах** (`server_e2e.rs`, `server_e2e/unavailable.rs`) — фикстуры доехали,
греп до них достаёт.

**Что будет потеряно при свёртке** (одно предложение):
Процессуальное правило для будущих пакетов — чужая краснота от смены фикстуры
лечится перенацеливанием стражей, а не правкой теста, — останется неоплаченным
прецедентом, который каждой следующей нарезке придётся открывать заново.

## Рулинги, чей дом — КОД, а не спека

- **1729: Р24** → `xtask/src/codegen/derive_floor.rs:15-39` →
  «Why it is unconditional rather than a tenth annotation. … `Debug`, `Clone`,
  `PartialEq` and `Eq` are properties of the Rust representation; the wire knows
  nothing of them and no reader's behaviour turns on them. … Why `Default` is
  NOT in the floor … "Does this type have a meaningful empty value" is a
  judgement about the type rather than a fact about its form … Why `Eq` is
  lawful today, said with its expiry rather than assumed: every schema in the
  tree carries zero float types … The first float in any schema takes that
  away».
- **1765: Р25** → `crates/vibe-index/src/cli/kinds.rs:21-35` →
  «Unfamiliar on the wire is normal life; unfamiliar in an argument is a user to
  tell (Б.1). … "package kind `{value}` is unknown to this build — known kinds:
  …"»; плюс `cli/list.rs:26-28` («The wire vocabulary is open, but the ARGUMENT
  speaks: a kind this build does not know is refused with a message, not
  filtered away in silence»).
- **1788: Р26** → `crates/vibe-wire/src/lib.rs:23-30` →
  «The crate stopped being generation-only when the consumers began re-exporting
  these types instead of duplicating them: a type this crate defines can carry
  its behaviour (vocabulary strings, emptiness, constructors, `finalise` passes)
  ONLY here — the orphan rule bars every other home.»
- **1805: Р27** → `formats/vocabularies.json:23` →
  «That same fact is why `x-rust-variants` exists here: from `kind/name` the
  generator mints a collision-suffixed identifier over the one it already gave
  `kind-name`, and no case rule produces a meaningful name — so the name is
  DECLARED rather than derived, keyed by the wire value both sides carry
  verbatim.»; плюс `xtask/src/codegen/domain_types/variants.rs:9-16` («The
  lookup runs one way only, and that direction is R16's law … Reasoning from the
  minted identifier would mean re-deriving a PascalCase rule plus a suffix …»).
- **1868: Р29** → `xtask/src/codegen/shared_module.rs:19-35` →
  «The mechanism, in the three phases the driver runs for the host home … 1.
  **The map.** … 2. **The shared module.** A synthetic JTD document … 3.
  **The replacement** (`rewire`).»
- **1921: Р33** → `crates/vibe-wire/src/behaviour/mod.rs:9-14` →
  «It lives in this crate because the orphan rule leaves no alternative: an
  inherent `impl` belongs in the crate that defines the type, and the consumers
  re-export these types instead of duplicating them, so that crate is this one.
  The layer is split by the same seam the generated side already has:
  vocabularies, per-record projections, and the records and aggregates
  themselves.»
- **1935: Р34** → `xtask/src/codegen/domain_types/rulings.rs:108` →
  `if let Some(ruling) = ruling_for(doc, "(the root)", &pascal_case(root_stem), schema)?` —
  механизм читает аннотацию и на КОРНЕ документа; сам принцип —
  `schemas/hello/e1/hello.jtd.json:3`: «the name is DECLARED rather than minted
  from the stem».
- **2145: Р36** → `crates/vibe-index/tests/round_trip_published.rs:8-12` →
  «The two inverted surfaces are still WRITTEN through their hand-written twins
  (`CapabilityRow` / `PurlRow`) and READ into the schema-generated `ByCap` /
  `ByPurl`, so each comparison is field by field — different structs by name,
  and that seam is exactly what the round-trip guards.»
- **2402: Р40** → `crates/vibe-index/src/index/memory.rs:404-408` →
  «A projection of the FORMAT REGISTRY, not of the journal (Р40): both numbers
  come from the generated `FormatId`, neither from a literal, and no clock
  enters — the handshake is a function of the registry and the binary, so two
  builds of one state produce the same bytes.»
- **2455: Р44** → `schemas/hello/e1/hello.jtd.json:27` →
  «`semver::Version` in code, declared by annotation (Р44, А.5а) — the tolerance
  the registry's `foreign_parsers = "many"` delivers covers unknown KEYS, never
  a lie about a known key's type.»; и `:84` → «Spelled `"type": "timestamp"`
  and not `"type": "string"` for the reason Р11 already settled … a bare string
  would give a date a SECOND legal spelling in the system, and one thing is
  written one way.»
- **2467: Р45** → `schemas/hello/e1/hello.jtd.json:55` →
  «what Р45 forbids is the other side of the comparison — the CLIENT does not
  mint a constant of its own, it reads its epoch from `formats/REGISTRY.toml`
  through the generated `FormatId::epoch()`.»
- **2492: Р47** → `xtask/src/codegen/optional_shapes/emit.rs:297-304` →
  «The list is meant to be COMPLETE over those forms, and it had exactly one
  hole: JTD's `timestamp` renders as `DateTime<FixedOffset>`, and until a schema
  needed an OPTIONAL date no site ever asked. The two halves of this pass
  disagreed precisely there … this side had no class for its Rust spelling and
  refused rather than guess.»
- **2814: Р54.2** → `tools/self-check.sh:242-245` →
  «a named-exception list of that length rots within a week, so the form is a
  RATCHET: today's count is frozen per crate in `wire-derive-baseline.json` and
  any GROWTH goes red. New handwritten wire stops appearing silently; not one
  lawful line is declared a violation today.»
- **2845: Р54.4** → `crates/vibe-index/src/cli/dump.rs:44-49` →
  «The JSONL stream stays one shape — a `VersionEntry` per line: a line of any
  other shape in this stream is a break in the wire, and `dump` is bulk export,
  not an answer by NAME, which is whom PROP-044's no-silence law addresses. The
  unusable set is visible in `--format json`, and the loader has already named
  every such version with a WARN line on stderr in this very run.»

## Контроли

1. **Класс `spec` с дословной цитатой — четыре рулинга:**
   - 1908: Р32 → PROP-005 §2.12 `#TWO-SHAPES-STAY-HAND-WRITTEN-AND-SAY-WHY`:
     «Two shapes stay hand-written, and each says why. `Repomd` /
     `RepomdFileEntry` — its `size` is a `u64` where the schema language reaches
     only `u32` (an open owner fork, `BACKLOG.md` B-056)».
   - 1949: Р35 → PROP-005 §2.12 `#RUST-TYPES`: «`VersionEntry` comes from the
     shared `version_entry` vocabulary; `NameEntry` / `PackageEntry` /
     `Tombstone` from `schemas/index/e1/by_name.jtd.json`; `BindingSite` from
     `by_purl`.»
   - 2871: Р55.1 → PROP-005 §2.19 `#WHICH-SURFACES-OWE-THE-ANSWER`: «every
     surface that COMPUTES an answer owes the refusal; a surface that serves a
     stored file verbatim does not.»
   - 2897: Р55.4 → PROP-005 §2.19 `#A-REFUSED-VERSION-IS-A-404-CARRYING-ITS-REASON`:
     «Over HTTP the status stays `404` and the body carries the reason. … an
     **extension member** carries the whole answer row».
2. **Класс `code` с дословной цитатой докблока** — 1729: Р24 →
   `xtask/src/codegen/derive_floor.rs:24-31`: «Why `Default` is NOT in the
   floor, which is the interesting half. "Does this type have a meaningful
   empty value" is a judgement about the type rather than a fact about its
   form … so `Default` belongs to the hand-written impls beside this tree.»
   (полный список код-рулингов с цитатами — секция выше).
3. **Контроль непустоты по каждому `none`** (§0.7):
   - 1880: Р30 — `epoch` в `formats/vocabularies.json` = 0; контроль
     `x-rust-type` в том же файле = **5**.
   - 1892: Р31 — рассуждение о классах трейтов = 0; контроль `PackageKind` в
     `crates/vibe-index/src` = **122 в 26 файлах**.
   - 2532: Р48 — правило в `spec/**` = 0; контроль `specmark::scope!`:
     `src` = **70 файлов**, `tests` = **0** (то самое единообразие).
   - 2834: Р54.3 — история дефекта = 0; контроль `unavailable` в
     `cli/get.rs` = **15**.
   - 2883: Р55.2 — `must_understand` в `crates/vibe-registry/src` = **0**;
     контроль тем же грепом в `crates/vibe-index/src` = **32 в 10 файлах**.
   - 2909: Р55.6 — правило в `spec/**` = 0; контроль `populated_state` в
     `crates/vibe-index/tests` = **25 в 2 файлах**.

## Счёт

Рулингов в сегменте (строки 1721–3001, по заголовкам, включая тире-формы Р36 и
Р37): **40**. Классы: **spec — 4** (Р32, Р35, Р55.1, Р55.4), **both — 16** (Р28,
Р37, Р38, Р39, Р41, Р42, Р43, Р46, Р49, Р50, Р51, Р52, Р53, Р54.1, Р55.3,
Р55.5), **code — 14** (Р24, Р25, Р26, Р27, Р29, Р33, Р34, Р36, Р40, Р44, Р45,
Р47, Р54.2, Р54.4), **none — 6** (Р30, Р31, Р48, Р54.3, Р55.2, Р55.6).
Все четыре класса представлены — моноклассового прохода нет.
