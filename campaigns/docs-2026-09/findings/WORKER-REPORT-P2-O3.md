# WORKER-REPORT-P2-O3 — индекс, гейт, скаффолд и прогрев для `doc`

Пакет: `campaigns/docs-2026-09/findings/PACKET-P2-O3.md` (A2.3, A2.5, A2.6,
A2.7 + хвост P2-O5). Ветка `research-preview-1-docs`, без push. Дата:
2026-09-12.

## Коротко для оркестратора

Все четыре атома и хвост сделаны и закоммичены, пять коммитов. Незакоммичен
у меня только перегенерированный `specmap.json` (0 suspects) — в нём рёбра
сразу нескольких параллельных воркеров, забирать их в свой атомарный коммит
было бы неверно; это решение P2-O1, и я его продолжаю.

По пути найден и починен **латентный дефект кодгена** (две пассы читали
`#[serde(rename)]` шаблоном, которого генератор не пишет). Без починки A2.3
физически не собирался: `abstract` — зарезервированное слово Rust, и это
первое в дереве *необязательное* поле с экранированным идентификатором.

## Хэши и subject'ы

| Атом | Коммит | Subject |
|---|---|---|
| A2.3 | `cb041f5d` | `feat(index): index doc packages with their relations and cards` |
| A2.5 | `9993148f` | `feat(check): gate doc packages, translations and media on their contract` |
| A2.6 | `09afc33a` | `feat(init): scaffold doc packages and their translations` |
| A2.7 | `93b0d7ff` | `feat(cache): warm a doc package's subjects and source so citations resolve offline` |
| хвост P2-O5 | `153e68d9` | `chore(cli): let the pivot read documentation and observe the manual` |

## A2.3 — индекс: связи и карточка (`cb041f5d`)

**Файлы.** `formats/vocabularies.json`; кодген
`crates/vibe-wire/src/generated/{shared,index/e1/{entry,by_name},index_cli/e1/get_report,index_http/e1/package_versions_response,journal/e1/journal}/mod.rs`;
`crates/vibe-wire/src/behaviour/{projections,records}.rs`;
`crates/vibe-index/src/types/entry/documentation.rs` (новый),
`types/entry/mod.rs`, `types/mod.rs`, `types/entry/tests.rs`;
`crates/vibe-index/src/scanner/{manifest,org_walk}.rs`,
`crates/vibe-index/src/cli/add.rs`,
`crates/vibe-index/src/index/{by_name,inverted,primary,memory/tests}.rs`;
восемь фикстурных файлов `crates/vibe-index/tests/**`;
`formats/corpora/index/e1/**` (журнал + перепроекция);
`xtask/src/codegen/{empty_policy/emit.rs,optional_shapes/emit.rs,optional_shapes/tests.rs,optional_shapes/tests/stitch.rs}`.

**Сделано.** `version_entry` расширен ровно теми шестью полями, которые
называет `##REL-INDEX-FIELDS`: `title`, `abstract`, `documents`,
`documentation`, `translates`, `media`. Четыре новых вокабуляра
(`documents_entry`, `documentation_entry`, `translates_entry`,
`media_entry`), `cargo xtask codegen`, `is_empty`/`Default` рядом с
сиблингами в `behaviour/projections.rs`, четыре проекции-хелпера в сканере
(`documents_from`, `documentation_from`, `translates_from`, `media_from` —
последняя нормализует разделители пути к `/`, как `boot_snippet_from`), оба
литерала записи (`org_walk.rs`, `cli/add.rs`) и `VersionEntry::minimal`.
Пустая секция — отсутствие на проводе, как у всех сиблингов.

**`known()` и CLI `--kind` уже знали `doc`/`app`** — это сделал A2.2
(`crates/vibe-wire/src/behaviour/vocabularies.rs::known()` → 8,
`crates/vibe-index/src/cli/kinds.rs::parse_kind_flag` перечисляет их в
отказе). Проверено чтением, не переделывал.

**Обратные запросы в индексе не заведены** — по A0.21 и
`##REL-REVERSE-QUERIES-SITE-SIDE`: «кто документирует X» и «какие переводы
есть у Y» — одна свёртка `primary.jsonl` в памяти, индекс не получает ни
маршрутов, ни папок. Это пинуется тестом
`reverse_documentation_questions_are_a_fold_and_add_no_files`, который
проверяет и свёртку, и **отсутствие** `by-documents/` / `by-translates/` в
записанном дереве.

### Развилка: `lang` в записи индекса — закрыта отказом

Пакет (и план A2.3) называют `lang` среди полей записи. Я его **не завёл**,
и это не пропуск:

- `##REL-INDEX-FIELDS` перечисляет поля поимённо, и `lang` там нет:
  «`title`, `abstract`, `documents`, `documentation`, `translates`,
  `media`»;
- `##LOC-LANGUAGE-FIELD`: «язык `doc`-пакета — существующий
  `[i18n].canonical`; **отдельного поля `lang` нет**»;
- решение A2.4 (обязательное для меня) заводит `lang` в манифесте **только
  чтобы отказать** и никогда не сериализует его.

Индексировать было нечего: язык уже ехал в записи как `i18n.default`
(`i18n_from` кладёт туда `canonical`). Тест
`a_doc_manifest_projects_its_card_and_its_relations` это утверждает явно.
`translations` не заведён по той же причине (`##LOC-NO-TRANSLATIONS-TABLE`,
и в `REL-INDEX-FIELDS` его нет).

### Аномалия продукта, найденная и **починенная**: `rename_wire` в двух пассах кодгена

`cargo xtask codegen` упал:

```
Error: …\shared\mod.rs:2430: the generated field `abstract_` is an optional shape
keyed `abstract_`, which no member of the schema describes.
```

Причина. jtd-codegen экранирует ключевое слово и пишет
`#[serde(rename = "abstract")]` + `pub abstract_: Option<Box<String>>`.
Пассы ключуют поле по *wire-имени*, то есть по сохранившемуся `rename`. Но
их хелперы `rename_wire` в `optional_shapes/emit.rs:325` и
`empty_policy/emit.rs:325` искали подстроку `"]`, тогда как генератор пишет
`")]` — в `snake_case.rs:212` (их собственный «близнец», так и названный в
доккомментариях) шаблон правильный. То есть обе копии **никогда** не
находили rename и молча ключевали поле экранированным идентификатором.

Почему это было невидимо. Единственный сохранившийся rename в дереве до
сегодня — `ref_` в `registry_sync_report`, и он на **обязательном** поле,
которое ни одна из этих двух пасс не ключует. `abstract` — первое
*необязательное* поле этого класса.

Починка: обе копии приведены к эмиссии генератора. Регрессия закреплена
тестом `a_keyword_renamed_optional_is_keyed_by_its_wire_name`
(`xtask/src/codegen/optional_shapes/tests.rs`) — обязательная половина
класса (`ref_`) уже была покрыта соседним тестом, добавлена та, которая
вообще доходит до этой пассы. Перепись сайтов
`the_real_vocabulary_home_walks_to_its_sites` поправлена 44 → 53 с
перечислением девяти новых.

### Корпус пополнен: `doc` и `app` на настоящем проводе

P2-O1 отложил образцы `doc`/`app` в корпуса до A2.3 — сделано. В журнал
`formats/corpora/index/e1/state/journal/2026-08.ndjson` добавлены три
публикации: `doc`-руководство с карточкой и ребром `documents`, его русская
адаптация с `translates` и **не-ASCII** `abstract` (то есть экранированный
`abstract_` доказан на committed-байтах), и `app`, указывающий обратно через
`documentation`. Корпус перепроецирован: 3 пакета/5 версий → 6 пакетов/8
версий, `primary.jsonl`, `.gz`, `repomd.json`, три новых `by-name/`.

Механика перепроекции, для протокола: **отгружаемого глагола для этого
нет** — `vibe-index rebuild` умеет только `--check` и отказывает словами
«repairing the catalog from its journal in place is a separate decision»
(`PROP-005#cli`). Проекция выполнена одноразовым тестовым файлом
`crates/vibe-index/tests/zz_corpus_regen.rs` (`journal::replay` →
`project` → `write_to(corpus)`), прогнанным как `cargo test -p vibe-index`
и **удалённым сразу после**; в коммит он не входит. Прогон именно
`-p vibe-index` (без `xtask` в графе сборки) обязателен — иначе `.gz`
получит другой бэкенд (аномалия P2-O1 №1, воспроизведена ниже).

**Вердикт `wire-diff`.** Шаг 1 падает по той же досессионной причине
(gzip-бэкенд при `xtask` в графе), до вердикта не доходит. Сдвиг
наблюдаемой поверхности атома — один файл, `formats/vocabularies.json`;
`formats/EPOCHS.toml` держит `public = false`, `break_window_open = true`,
значит по таблице `xtask/src/wire_diff.rs` это **green, REPORTING**, и
break-заметка не нужна.

## A2.5 — гейт `vibe check` (`9993148f`)

**Файлы.** `crates/vibe-check/src/checks/doc_package_contract.rs` +
`doc_package_contract/tests.rs`, `checks/doc_translation.rs` +
`doc_translation/tests.rs`, `checks/doc_media.rs` + `doc_media/tests.rs` +
`doc_media/image.rs` + `doc_media/image/tests.rs`; регистрация в
`checks/mod.rs` и `lib.rs`; `tests/check_cells_oracle.rs`; `Cargo.toml`
(+`toml`).

**Три ячейки, по одной на существительное из subject'а** — «doc packages,
translations and media». Семь правил пакета читают один и тот же манифест и
распадаются ровно на эти три предмета; семь `CheckId` на один контракт
сделали бы вывод `vibe check` труднее для чтения, а не легче. Закон
`checks/mod.rs` («one module per `CheckId`… never a sibling cell») соблюдён.

| Ячейка | Правила | Якорь |
|---|---|---|
| `doc_package_contract` | `[boot_snippet]` / `[[binary]]` / `[[mcp_server]]` в `doc` — ошибка (каждая секция своя находка); нет `README.md` — ошибка; `documentation.primary` в чужую группу — предупреждение (в пакете **любого** вида) | `##KIND-DOC-MUST-NOT-EXECUTE`, `##KIND-DOC-MUST-DOCUMENT`, `##REL-DOCUMENTATION-UNVERSIONED` |
| `doc_translation` | `[translates]` без **написанного** `[i18n].canonical` — ошибка; `documents` перевода ≠ `documents` источника — ошибка; источник недоступен офлайн — предупреждение с причиной и рецептом | `##LOC-LANGUAGE-FIELD`, `##LOC-DOCUMENTS-MATCH` |
| `doc_media` | существование, сигнатура (PNG/JPEG/WebP; SVG — свой отказ «умеет нести скрипт»), пропорции и байтовый потолок по D-20; формат распознан, но заголовок не прочитан — предупреждение | `##CARD-MEDIA-SOURCE` |

**Развилка 1: `translates` без `lang` — переформулировано, не пропущено.**
Поля `lang` нет (A2.4). Эквивалент по норме: перевод обязан **написать**
`[i18n].canonical`, потому что умолчание — `en`, и молчание заставило бы
каждую адаптацию заявлять английский. Разобранный `Manifest` на этот вопрос
ответить не может (`#[serde(default)]` подставляет умолчание), поэтому
присутствие ключа спрашивается у **текста** файла — один поиск по пути,
никакой второй копии грамматики манифеста. Сообщение отказа называет поле,
которое работает, и говорит, что `lang` не существует: план кампании сам
велел авторам писать `lang`, и они будут.

**Развилка 2: три правила намеренно не продублированы.** `[[documents]]`,
`title` и `abstract` отвергает сама грамматика манифеста (A2.4,
`Manifest::validate`), с теми же якорями и с рецептом. `doc`-пакет без них
не разбирается вовсе, и `vibe check` уже краснеет через
`manifest_validity`. Вывести их заново в линтере значило бы завести вторую
копию грамматики — ровно та гниль, которую описывает история сканера
индекса («the pre-de-rot scanner hand-duplicated a `vibe.toml` parser…
and it rotted silently»). Поведение, которого требует пакет («без
`[[documents]]`, `title` или `abstract` — ошибка»), выполняется; тест
`the_manifest_grammar_still_refuses_a_doc_package_without_its_card`
закрепляет, **откуда** приходит отказ, чтобы ослабление заметили здесь.

**Наблюдение по пути:** `[[mcp_server]]` в `doc`-пакете до ячейки вообще не
доходит — грамматика отвергает его раньше, по PROP-027 («legal only in
`mcp`-kind packages»). Ветка в ячейке оставлена (правило принадлежит
`##KIND-DOC-MUST-NOT-EXECUTE`, чем бы оно ни обеспечивалось), тест
`an_mcp_server_is_refused_before_this_cell_ever_sees_it` фиксирует
фактический источник отказа.

**Чтение картинок — свой заголовочный ридер**, не декодер
(`doc_media/image.rs`, ~200 строк): сигнатуры PNG/JPEG/WebP, размеры из
IHDR / SOF / трёх контейнеров WebP (`VP8X`, `VP8 `, `VP8L`), SVG по тексту
(с BOM и с XML-прологом). Всё с проверкой границ: усечённый или враждебный
заголовок завершает разбор, а не паникует и не зацикливается — это пинуется
тестом `a_truncated_header_answers_none_instead_of_panicking`, который
режет фикстуру по каждому байту. Фикстуры строятся байтами в самом тесте,
не файлами.

**Приёмка пакета:**

```
$ vibe check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0
vibe check: clean — every check passed against
`C:\Users\olegc\git\v\vibevm-docs\vibevm\vibepacks\org.vibevm.core\vibevm-docs\v0.1.0`
CHECK_EXIT=0
```

## A2.6 — `vibe init --kind doc` (`09afc33a`)

**Файлы.** `crates/vibe-cli/src/commands/init/doc.rs` (новый) +
`init/doc/tests.rs`, `init/package.rs`, `init/mod.rs`,
`crates/vibe-cli/src/cli/pkg.rs`.

**Скаффолд.** `vibe.toml` с `kind = "doc"`, `title`, `abstract` (шаблон
четырёх вопросов — **комментарием**, чтобы автор, ответивший на них,
написал аннотацию, а не удалил непрочитанный lorem ipsum), `[i18n]
canonical = "en"`, заглушка `[[documents]]`, закомментированный `[media]` с
лимитами D-20; `README.md`; `vibevm/vibespecs/README.xml` с одним примером
страницы; `specmap.toml` (`scan_roots = []`, `spec_roots` — страницы);
`media/.gitkeep`. Boot-лейна нет вовсе. Повторный запуск ничего не
переписывает.

**Отличие от плана: `lang = "en"` → `[i18n] canonical = "en"`.** План A2.6
называет `lang`; поле отвергается A2.4, и скаффолд, пишущий его, выдавал бы
неразбираемый манифест. Тесты проверяют скаффолд **разбором**
(`Manifest::read`), а не сравнением строк — именно чтобы такое не проехало.

**`--translates <координата>`** зеркалит дерево источника **копированием**
страниц, а не генерацией пустых: `##LOC-MIRROR` требует совпадения путей,
якорей, идентификаторов и числа блоков, а копия — идеальное зеркало в день,
когда она сделана; переводчик заменяет прозу на месте, и зеркало живёт по
построению. `[[documents]]` источника копируются дословно (их сравнивает
гейт A2.5). Источник ищется там, где его можно честно найти офлайн:
`vibevm/vibepacks` проекта, затем машинный store.

**Язык берётся из имени пакета.** Официальный перевод публикуется как
`<docname>-<lang>` (`##LOC-OFFICIAL-TRANSLATION`), то есть соглашение уже
несёт тот единственный факт, ради которого иначе понадобился бы флаг. Имя,
не следующее соглашению, отвергается с объяснением соглашения; источник, до
которого не дотянуться, — с рецептом `vibe cache add`.

**B-134 починен по пути.** `--kind` не читался нигде: слот всегда минтился
`tool`, при том что `--help` обещал восемь видов. Теперь вид идёт в
манифест, в имя файла сниппета (`10-<kind>-<name>.md`) и в его категорию
(`flow`/`stack`/`app` имеют свою, остальные вносят как `tool` — ровно то,
что лейн и так предполагал); неизвестный вид отвергается тем единственным
списком, который определяет множество (`vibe_core::PackageKind::from_str`).

Живая проба (вне дерева, в scratch):

```
$ vibe init package org.acme/acme-docs --kind doc
  ✓ created  vibevm/vibepacks/org.acme/acme-docs/v0.1.0/vibe.toml
  ✓ created  …/README.md
  ✓ created  …/vibevm/vibespecs/README.xml
  ✓ created  …/specmap.toml
  ✓ created  …/media/.gitkeep
$ vibe check --path …/org.acme/acme-docs/v0.1.0
vibe check: clean — every check passed

$ vibe init package org.acme/review-flow --kind flow     # B-134
kind = "flow"          (было: tool)
10-flow-review-flow.md (было: 10-tool-review-flow.md)

$ vibe init package org.acme/acme-docs-ru --kind doc --translates org.acme/acme-docs
title = "Acme Docs (ru)"   [i18n] canonical = "ru"
[translates] package = "org.acme/acme-docs"  version = "^0.1"
vibevm/vibespecs/README.xml   (зеркало источника)
```

## A2.7 — прогрев замыкания (`93b0d7ff`)

**Файлы.** `crates/vibe-cli/src/commands/cache/add.rs`,
`crates/vibe-cli/tests/cli_cache.rs`.

**Сделано.** Прогрев `doc`-пакета добавляет в замыкание координаты из
`[[documents]]` и `[translates]` по их ограничениям версий
(`##REL-WARMUP-CLOSURE`). Обход идёт **до неподвижной точки**, а не один
раз: адаптация тянет источник, источник тянет свои предметы — уровень,
которого один проход не достигает. Внутри раунда работает существующий
`solve`; `[requires]` не переизобретается.

**Корень и производная координата разведены.** Корень, который назвал
пользователь, обязан разрешиться — отказ там остаётся отказом команды.
Производная координата решается по одной, и её неудача **записывается, а не
поднимается**: предметом документации может быть координата проекта,
которую не отдаст ни один реестр — хост сам такой (`##REL-HOST-SUBJECT`), —
и отказ там сделал бы руководство проекта непрогреваемым. Непрогретый
предмет называется вслух с причиной (и в человеческом выводе, и в
`unwarmed_subjects` JSON): одна цитата на тех страницах офлайн не
откроется, и молчание дало бы читателю узнать об этом в самолёте.

Живая проба на настоящем руководстве ядра:

```
$ VIBE_SETTINGS=<scratch> vibe cache add --offline "org.vibevm.core/vibevm-docs@0.1.0"
Pre-warming the machine store (<scratch>\cache)
  ✓ created  org.vibevm.core/vibevm-docs@0.1.0
  → documented subject `org.vibevm.core/vibevm@^1.0` not warmed — package
    `org.vibevm.core/vibevm` is not available in any configured registry …

1 fetched, 0 already present — nothing materialised into any project.
```

Положительный случай закрыт интеграционным тестом
`cache_add_warms_a_doc_packages_subjects_and_its_source` на фикстурном
`file://`-реестре: прогрев адаптации кладёт в store адаптацию, её источник,
общий предмет **и** зависимость предмета.

## Хвост P2-O5 (`153e68d9`)

1. **Аудитория.** `(expected user|author|dev)` устарело после допуска
   `agent`. Теперь сообщение читает набор из самого словаря
   (`Audience::ALL`), так что отстать снова не сможет; `--help` тоже
   поправлен.
2. **Словарь пивота.** `grounding.rs` читал все страницы под spec-диалектом.
   Какой словарь открыт, решает **вид содержащего пакета**, не файл
   (PROP-045 `##DOC-VOCAB-BY-KIND`), поэтому grounding поднимается к
   ближайшему манифесту и для `doc` берёт `Vocabulary::Doc` через
   `load_spec_text_with`. Ответ мемоизируется по каталогу пакета:
   руководство — сотни файлов и один манифест.
3. **Руководство в наблюдении.** `facts.toml` `include` получил
   `vibevm/vibepacks/org.vibevm.core/**` (обе сериализации, `.md` и
   `.xml`, по FORMAT NOTE B-107).

**Что при этом вскрылось и было доделано (X-024).** `[judging] exempt`
освобождал только от долга судейства, но не от `--exhaustive`. Это одно и
то же требование на шаг раньше: `--exhaustive` спрашивает маркер у каждой
прозаической единицы, долг спрашивает вердикт у каждого маркера.
Освобождение только второго оставляло руководство вне долга и **всё равно**
красным на гейте, считающем сырьё этого долга — **1560 ошибок**, — и
«observed, never judged» оказалось бы непригодно для того жанра, ради
которого написано. Теперь `--exhaustive` уважает тот же ключ; исключение
компилируется один раз, в grounding, и едет на `Ground`. Поведение
закреплено тестом
`a_judging_exemption_frees_a_file_from_exhaustive_too`
(`crates/vibe-cli/tests/cli_facts.rs`) с контролем «то же дерево без
исключения краснеет» и с утверждением, что файл остаётся **наблюдаемым**.

```
$ vibe facts check --exhaustive
progress check: clean (380 files, 24 warning(s))
FACTS_EXIT=0
```

(24 предупреждения — те же `FoldLossy`, что печатал прогон до правки
`facts.toml`: «1560 error(s), 24 warning(s)». Корпус вырос на страницы
руководства — 380 наблюдаемых файлов; прежнего числа файлов у меня
измеренным нет, прогон до правки печатал только счётчики ошибок.)

## Вывод гейтов, дословно (на `153e68d9`)

```
$ cargo fmt --all --check
FMT_EXIT=0

$ cargo build --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 49.73s
BUILD_EXIT=0

$ cargo test -p vibe-index -p vibe-wire -p vibe-check
77 test binaries, all `test result: ok`; zero FAILED, zero `failures:`

$ cargo test -p vibe-cli --bins commands::init
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 719 filtered out

$ cargo test -p vibe-cli --test cli_cache
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p vibe-cli --test cli_facts
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p xtask --bins
test result: ok. 293 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo clippy -p vibe-core -p vibe-index -p vibe-wire -p vibe-check \
                -p vibe-cli -p progress-core -p xtask --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 49.00s
CLIPPY_EXIT=0

$ cargo xtask specmap
specmap: wrote specmap.json (7930 spec units, 3462 tagged code items, 3007 edges,
                             0 suspects, 31 warnings).
specmap: ratchet gate — 0 gated orphan(s), 0 dispositioned (6 crate(s) exempt).
specmap: resolve gate — 0 unresolved host edge(s), 35 non-host edge(s) …

$ cargo xtask check-codegen
xtask check-codegen: clean.
CC_EXIT=0

$ vibe check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0
vibe check: clean — every check passed
CHECK_EXIT=0

$ vibe facts check --exhaustive
progress check: clean (380 files, 24 warning(s))
FACTS_EXIT=0
```

## Что не сделано и почему

1. **`specmap.json` перегенерирован, но НЕ закоммичен** — 0 suspects, гейт
   пройден; в файле рёбра сразу нескольких параллельных воркеров. Коммитит
   центральная сессия, когда волна осядет (продолжение решения P2-O1 п. 4).
2. **`cargo xtask wire-diff` до вердикта не дошёл** — падает на шаге 1 по
   досессионной причине (аномалия №1 ниже). Вердикт выведен из таблицы
   `wire_diff.rs` и флагов `formats/EPOCHS.toml` вручную; сдвиг
   наблюдаемой поверхности — один файл `formats/vocabularies.json`.
3. **`formats/breaks/006.md` не создан** — не требуется: `public = false`.
4. **`[i18n].available` у `doc`-пакета не проверяется.**
   `##LOC-LANGUAGE-FIELD` говорит «пуст по построению: перевод — другой
   пакет», то есть непустой `available` в `doc`-пакете — нарушение. Правила
   нет в списке пакета, поэтому я его не открывал. Кандидат в
   `doc_translation`, четыре строки.
5. **`vibe check --path <пакет>` не находит источник перевода, лежащий в
   проекте-родителе.** Ячейка ищет `vibevm/vibepacks` относительно
   *проверяемого корня*, а корень — сам пакет. Поведение честное (находка —
   предупреждение «правило не проверено, вот рецепт», а не ложный зелёный),
   но из корня проекта тот же перевод проверился бы полностью. Починка —
   подъём по предкам до каталога, содержащего `vibevm/vibepacks`; не делал,
   потому что это не правило пакета, а эвристика поиска.
6. **Отчёт по `[media]` не проверяет, что картинка лежит внутри пакета** —
   это форма пути, её уже отвергает `Manifest::validate` (A2.4,
   `##CARD-MEDIA-SOURCE`), и ячейка не дублирует грамматику по той же
   причине, что и `title`/`abstract`.

## Аномалии продукта, найденные по пути

1. **Подтверждена аномалия P2-O1 №1** (golden-корпус и `xtask` в одном
   графе сборки). После пополнения корпуса воспроизводится дословно:

   ```
   $ cargo test -p vibe-index --test golden_corpus            → ok
   $ cargo test -p vibe-index -p xtask --test golden_corpus   → FAILED
     `primary.jsonl.gz` differs (binary): committed 1879 byte(s), projected 1891
     `repomd.json` differs first at line 90
   ```

   Committed-байты соответствуют бэкенду `-p vibe-index` — тому же, что был
   до меня; свойство сохранено, не введено.
2. **`vibe-index rebuild` не умеет чинить каталог** (только `--check`), а
   отгружаемого глагола перепроекции в дереве нет вовсе. Пополнить
   golden-корпус можно лишь одноразовой машинерией (см. A2.3). Стоит либо
   завести `rebuild --write` с явным подтверждением, либо
   `cargo xtask reproject-corpus`.
3. **`cargo xtask check-codegen` дважды падал транзиторно** с
   «installing fresh generated tree … Access is denied. (os error 5);
   restored the complete old tree» — параллельный воркер держал файл в
   `crates/vibe-wire/src/generated`. Откат корректен (дерево
   восстановлено), повтор проходит; но на CI это будет мигающий гейт.
   Стоит ретраить установку дерева.
4. **`crates/vibe-cli/tests/cli_search.rs` — те же два красных теста**, что
   у P2-O1 (`search_without_full_scan_keeps_unconfigured_status`,
   `search_full_scan_finds_matching_packages_in_github_org`). К моим
   правкам отношения не имеют: про конфигурацию реестров.
5. **`ctx.step()` в `output.rs` помечен `#[allow(dead_code)] // used by
   install`** — теперь его использует и `cache add`. Комментарий устарел на
   одно слово; не правил, чтобы не трогать чужой файл ради запятой.

## `git status --short` в конце

```
 M Cargo.lock
 M crates/vibe-doc/src/html.rs
 M crates/vibe-doc/src/lib.rs
 M crates/vibe-doc/tests/fixture/manual/vibevm/vibespecs/guide/every-block.xml
 M crates/vibe-doc/tests/golden/guide-every-block.html
 M specmap.json
?? crates/vibe-doc/src/html/emit.rs
?? crates/vibe-doc/src/md.rs
?? crates/vibe-doc/src/md/
?? crates/vibe-doc/src/numbering.rs
?? crates/vibe-doc/src/numbering/
?? crates/vibe-doc/src/xml.rs
?? crates/vibe-doc/src/xml/
?? crates/vibe-doc/tests/golden/guide-every-block.md
?? crates/vibe-doc/tests/golden/guide-every-block.numbered.html
?? crates/vibe-doc/tests/golden/guide-every-block.xml
?? crates/vibe-doc/tests/projections.rs
```

Из этого моё — **только `specmap.json`** (см. «Что не сделано» п. 1) и сам
этот отчёт. Всё остальное принадлежит параллельному воркеру крейта
`vibe-doc`; я этих файлов не трогал и не стейджил. Файлы тестовых фикстур
lock-файлов третьего воркера уже закоммичены им (`b73195fd`).
