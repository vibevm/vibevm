# WORKER-REPORT-P2-O1 — виды `doc` и `app`, wire-словарь, поля манифеста

Пакет: `campaigns/docs-2026-09/findings/PACKET-P2-O1.md` (A2.1, A2.2, A2.4).
Ветка `research-preview-1-docs`, без push. Дата: 2026-09-12.

## Коротко для оркестратора

Все три атома сделаны и закоммичены. Незакоммиченных правок у меня в дереве
нет. Один коммит из трёх **потерян как отдельный коммит**: пока я его
стейджил, соседний воркер сделал `git commit`, забрав весь индекс — содержимое
A2.2 целиком лежит внутри **чужого** коммита `005ee978`
(`docs(vibevm-docs): make the glossary and the formats page readable by the
pivot`). Требуемого `feat(wire): widen package_kind vocabulary for doc and app`
в истории нет. Историю я не переписывал — это решение центральной сессии.

## Хэши и subject'ы

| Атом | Коммит | Subject |
|---|---|---|
| A2.1 | `b18453f2` | `feat(core): teach PackageKind the doc and app kinds` |
| A2.2 | **`005ee978`** (чужой коммит, см. выше) | содержимое внутри `docs(vibevm-docs): make the glossary and the formats page readable by the pivot` |
| A2.4 | `29351a68` | `feat(core): carry documentation relations, locales and the card in the manifest` |

## A2.1 — виды `doc` и `app` в ядре (`b18453f2`)

**Файлы.** `crates/vibe-core/src/package_ref/kind.rs`,
`crates/vibe-core/src/package_ref/tests.rs`, `schemas/extensions_report.jtd.json`,
`crates/vibe-wire/src/generated/extensions_report/mod.rs` (кодген),
`crates/vibe-cli/src/commands/extensions.rs`,
`crates/vibe-index/src/scanner/manifest.rs`, `crates/vibe-install/src/error.rs`,
`crates/vibe-install/src/plan.rs`, `crates/vibe-install/tests/doc_kind_refusal.rs`
(новый), `crates/vibe-cli/src/commands/bin.rs`.

**Сделано.**

- `PackageKind::{Doc, App}` с doc-комментариями по PROP-057 `##KIND-DOC-LEAD`
  и `##KIND-APP-VS-TOOL`; `as_str`, `ALL` (8), `FromStr`. Заодно поправлен
  устаревший доккомментарий «One of the four installable package kinds» (факт
  из A0.1, свидетельства): теперь «eight» плюс ссылка на `##KIND-CODE-LAW`.
- `PackageKind::is_read_only()` — одна точка истины для «этот вид читают, а не
  ставят». Сейчас это только `Doc`.
- **`vibe install` отказывает doc-пакету.** Место — `crates/vibe-install/src/plan.rs`,
  сразу после цикла fetch: вид пакета лежит внутри его собственных байтов, так
  что раньше него он неизвестен вообще, а позже уже пишется слот. Проверяется
  **весь решённый граф**, не только корни: doc-пакет, до которого дотянулись
  через чужой `[requires]`, — та же ошибка на уровень дальше, и назвать корень
  значило бы назвать не тот пакет. Новый вариант ошибки
  `vibe_install::Error::DocNotInstalled`; текст содержит координату,
  `vibe cache add <coord>` и адрес
  `spec://org.vibevm.core/vibevm/common/PROP-057#KIND-DOC-NOT-INSTALLED`.
- **`vibe bin exec` отказывает app-пакету** (`crates/vibe-cli/src/commands/bin.rs`,
  `refuse_app_dispatch`): вид читается из манифеста слота (у `DeclaredBinary`
  уже есть `slot`, менять структуру и её конструкторы не пришлось). Нечитаемый
  манифест слота — не диагноз этой команды: возвращается `Ok`, диспетчеризация
  идёт прежним путём. Адрес `…PROP-057#KIND-APP-VS-TOOL`.
- `app` ведёт себя как `tool` везде, где спека не различает: специально
  проверено тестом, что `app` **не** попадает под install-отказ.
- Перепись A0.1 подтверждена компилятором. Реально красных исчерпывающих
  матчей оказалось **три** (не пять): `kind.rs`, `scanner/manifest.rs` и два
  матча в `extensions.rs`; `vibe-wire/src/behaviour/vocabularies.rs`,
  `generated/shared/mod.rs` и `tests/open_vocabulary.rs` красными в A2.1 не
  стали — они матчат по *открытому* wire-типу, который в этом атоме ещё не
  расширялся (расширен в A2.2).

**Решения и развилки.**

1. **`schemas/extensions_report.jtd.json` расширен в A2.1, а не в A2.2.**
   Два матча в `extensions.rs` конвертируют `vibe_core::PackageKind` в
   `ManifestKind` и `ReportPackageKind` — это **сгенерированные закрытые**
   enum'ы из локальных словарей той схемы (`x-vocabulary: "closed"`), у них нет
   `Unknown`. Без расширения схемы A2.1 физически не собирается, а ветка
   «правильного поведения» там — точное отображение, а не заглушка. Схема
   расширена, `cargo xtask codegen` прогнан. Побочно найдено: эти два словаря —
   **третья копия** набора видов, и её **не сторожит** `vocabulary_parity`
   (в списке `linked_schema` этой схемы нет). Занесено в аномалии.
2. **Мост через `Unknown` в `scanner/manifest.rs`.** В A2.1 ветки
   `CorePackageKind::Doc/App` отображались в `PackageKind::Unknown("doc"/"app")`.
   Это не заглушка и не ложь на проводе: открытый словарь для того и открыт —
   `Unknown("doc")` пишет ровно те байты, что напишет именованный вариант после
   A2.2 (PROP-044 §4.2a). В A2.2 мост снят, ветки стали именованными.
   Промежуточное состояние коммита A2.1 корректно и на проводе, и в типах.

**Тесты.** `kind_names_doc_and_app`, `only_doc_is_read_instead_of_installed`
(`crates/vibe-core/src/package_ref/tests.rs`);
`installing_a_doc_package_is_refused_with_the_warm_up_hint`,
`installing_an_app_package_is_not_refused`
(`crates/vibe-install/tests/doc_kind_refusal.rs`, новый интеграционный файл —
крейт держит `[lib] test = false`);
`an_app_package_is_never_dispatched_by_bin_exec`,
`every_other_kind_still_dispatches` (`crates/vibe-cli/src/commands/bin.rs`).

## A2.2 — wire-словарь (содержимое в `005ee978`)

**Файлы.** `formats/vocabularies.json` (+ `doc`, `app`),
`crates/vibe-wire/src/generated/shared/mod.rs` и
`crates/vibe-wire/src/generated/index_cli/e1/list_report/mod.rs` (кодген),
`schemas/index_cli/e1/list_report.jtd.json`,
`crates/vibe-wire/src/behaviour/vocabularies.rs`,
`crates/vibe-wire/tests/open_vocabulary.rs`, `crates/vibe-core/src/error.rs`,
`crates/vibe-cli/resources/package-tree.schema.v1.json`,
`crates/vibe-cli/src/commands/init/prompts.rs`, `crates/vibe-cli/src/cli.rs`,
`crates/vibe-cli/src/cli/pkg.rs`, `crates/vibe-index/src/cli/list.rs`,
`crates/vibe-index/src/cli/search.rs`, `crates/vibe-index/src/scanner/manifest.rs`,
`crates/vibe-mcp/src/skill_template.md`, `xtask/src/batch_review/refs.rs`,
`xtask/src/codegen/vocabulary/tests.rs`.

**Сделано.** Расширен объявленный домен и **каждая** рукописная копия, которую
поимённо сторожит `crates/vibe-wire/tests/vocabulary_parity.rs`: сгенерированный
открытый enum через `cargo xtask codegen`, поведенческий слой рядом с ним
(`as_str` / `known()` → 8 / `FromStr`, число в тесте 6 → 8), сообщение
`BadPackageKind` и его doctest, JSON-схема дерева пакетов, интерактивный
`vibe init`, четыре текста справки CLI, шаблон скилла MCP, две фикстуры xtask,
прозаическое утверждение открытого словаря в nullable-схеме `list_report`.
Порядок домена — `flow, feat, stack, tool, mcp, lang, doc, app` — совпадает с
уже поправленным (фаза 1) PROP-000.

**Корпуса — НЕ пополнены, осознанно.** См. «Что не сделано».

**Break-заметки нет, и она не требуется.** `formats/EPOCHS.toml`:
`public = false`, `break_window_open = true`. По таблице вердиктов
`xtask/src/wire_diff.rs` строка `public = false` + непустой сдвиг = **green,
REPORTING**. Сдвиг наблюдаемой поверхности (щуп шага 2, прогнан руками, потому
что шаг 1 падает по чужой причине — см. аномалии):

```
$ git diff --exit-code --name-only HEAD -- schemas/ formats/
formats/vocabularies.json
schemas/index_cli/e1/list_report.jtd.json
```

## A2.4 — поля манифеста (`29351a68`)

**Файлы.** `crates/vibe-core/src/manifest/package/documentation.rs` (новый),
`crates/vibe-core/src/manifest/package.rs`,
`crates/vibe-core/src/manifest/package/visibility.rs` (`ManifestWire` + оба
`TryFrom`), `crates/vibe-core/src/manifest/document.rs`,
`crates/vibe-core/src/manifest/document/validation.rs`,
`crates/vibe-core/src/manifest/mod.rs`,
`crates/vibe-core/src/manifest/document/tests_documentation.rs` (новый),
`crates/vibe-core/src/manifest/package/tests.rs`,
`crates/vibe-check/src/checks/boot_directory.rs`,
`crates/vibe-install/tests/doc_kind_refusal.rs`.

**Форма (по `REL-FIELD-PLACEMENT`, рекомендация в силе).** Верхний уровень:
`[[documents]]`, `[documentation]`, `[translates]`, `[media]`. В `[package]`:
`title`, `abstract`. Типы: `DocumentsDecl`, `DocumentationDecl`,
`TranslatesDecl`, `MediaDecl` — каждый со своим `deny_unknown_fields`, как все
семь существующих package-ролевых структур; новые поля заведены на **обеих**
структурах верхнего уровня и в **обоих** направлениях `TryFrom` (A0.3 п.1).

**Главная развилка — `lang` и `[translations]`: закрыта отказом, не полем.**
План A2.4 называет `lang` (BCP-47, default `en`) и `translations`. PROP-057
§5 со статусом `spec/done` говорит обратное и прямо:

- `LOC-LANGUAGE-FIELD`: «The language of a `doc` package is the existing
  `[i18n].canonical` of PROP-003 §2.7 (default `en`); **there is no separate
  `lang` field**.»
- `LOC-NO-TRANSLATIONS-TABLE`: «**The source stores no list of its
  translations.** Which translations a documentation has, the site and the
  local reader compute from the `translates` edges at every render.»

PROP-057 — норма пакета, и он новее плана; вдобавок сам пакет называет эти два
якоря среди тех, что **цитируют ошибки**, — а ошибка, цитирующая
`LOC-LANGUAGE-FIELD`, может быть только отказом. Эталонный манифест руководства
это подтверждает: в нём `[i18n].canonical = "en"`, а не `lang`.

Решение: оба ключа **читаются, чтобы отказ мог назвать поле, которое
работает**. `lang` заведён на `PackageMeta` (общей для домена и провода) и
отвергается в `Manifest::validate`; `[translations]` заведён **только** на
`ManifestWire` (доменный `Manifest` его не несёт) и отвергается в
`TryFrom<ManifestWire> for Manifest`. Ни один не сериализуется. Причина не
косметическая: план кампании сам велел авторам писать `lang`, так что этот ключ
будут писать, а голое serde'шное «unknown field `lang`» оставляет автора
гадать, какой из тридцати ключей манифеста отвечает за язык.

**Проверки и их якоря** (все имена проверены по тексту PROP-057):

| Правило | Якорь |
|---|---|
| `doc` без `[[documents]]` | `#REL-DOCUMENTS-REQUIRED` |
| `[[documents]]` не в `doc`-пакете | `#KIND-DOC-MUST-DOCUMENT` |
| координата не `<group>/<name>` (версия, `kind:`) | `#REL-DOCUMENTATION-UNVERSIONED` |
| `primary` повторён в `official` | `#REL-DOCUMENTATION-UNVERSIONED` |
| `version` не semver-ограничение | `#REL-DOCUMENTS-REQUIRED` |
| `[translates]` не в `doc`-пакете | `#LOC-PACKAGE-PER-LANGUAGE` |
| `doc` без `title` | `#CARD-TITLE` |
| `doc` без `abstract`, пустой `abstract`, `abstract` > 1000 | `#CARD-DESCRIPTION-AND-ABSTRACT` |
| путь `[media]` выходит за пакет | `#CARD-MEDIA-SOURCE` |
| `lang` | `#LOC-LANGUAGE-FIELD` |
| `[translations]` | `#LOC-NO-TRANSLATIONS-TABLE` |

`abstract` считается в `char`, а не в байтах — иначе русская адаптация молча
получила бы вдвое меньше места, чем английский источник. Существование,
сигнатура формата, пропорции и размер картинок — это гейт (A2.5), здесь
проверяется только форма пути.

**Одна правка за пределами `vibe-core`, вынужденная гейтом атома.**
`vibe check` на манифесте руководства давал две находки: `manifest_validity`
(её чинит этот атом) и `boot_directory` — «`vibevm/vibespecs/boot/` is missing».
Doc-пакет **обязан** не иметь boot-лейна (`KIND-DOC-MUST-NOT-EXECUTE`, §14
«Documentation never enters a boot lane»), то есть ячейка требовала ровно того,
что запрещает вид. Добавлено одно исключение (`is_doc_package`) плюс тест с
контролем на `flow`. Это решение **есть в PROP-057**, поэтому развилкой не
считается; списка ячеек A2.5 оно не трогает — там этого пункта нет.

**Тесты** (`crates/vibe-core/src/manifest/document/tests_documentation.rs`, 15
штук): разбор всей поверхности целиком, round-trip через провод,
`[documentation]` в пакете любого вида, все отказы выше, отдельная проверка
границы `abstract` в 1000 символов кириллицей (ровно лимит — проходит, +1 —
отказ), и «карточка остаётся необязательной вне документации». Плюс
`a_doc_package_owes_no_boot_directory` в `vibe-check` и юнит-тесты форм
координаты / ограничения версии / пути в самом новом модуле.

## Вывод гейтов, дословно

### A2.1 (на коммите `b18453f2`)

```
$ cargo fmt --all --check
FMT_EXIT=0

$ cargo build --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 54.29s
BUILD_EXIT=0

$ cargo test -p vibe-core
test result: ok. 475 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.09s
test result: ok. 188 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.19s

$ cargo test -p vibe-install --test doc_kind_refusal
running 2 tests
test installing_a_doc_package_is_refused_with_the_warm_up_hint ... ok
test installing_an_app_package_is_not_refused ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

$ cargo test -p vibe-cli an_app_package_is_never_dispatched / every_other_kind_still_dispatches
test commands::bin::tests::an_app_package_is_never_dispatched_by_bin_exec ... ok
test commands::bin::tests::every_other_kind_still_dispatches ... ok

$ cargo clippy -p vibe-core -p vibe-index -p vibe-install -p vibe-cli -p vibe-wire --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 55.55s
CLIPPY_EXIT=0
```

Ожидаемо красный между A2.1 и A2.2 (межатомный часовой, он же был красным ДО
моего первого коммита — см. «базовая линия» ниже):

```
$ cargo test -p vibe-wire --test vocabulary_parity
package_kind vocabulary drift from formats/vocabularies.json; every lagging copy is named:

enum source: vibe_core::PackageKind::ALL [crates/vibe-core/src/package_ref/kind.rs]
    expected (domain order): ["flow", "feat", "stack", "tool", "mcp", "lang"]
    actual:                  ["flow", "feat", "stack", "tool", "mcp", "lang", "doc", "app"]

PROP-000 package identity list [vibevm/vibespecs/common/PROP-000.xml]
    missing exact enumeration: kind ∈ {flow, feat, stack, tool, mcp, lang}

VIBEVM-SPEC §4.1 installable-kind headings [VIBEVM-SPEC.md]
    expected (set): {"feat", "flow", "lang", "mcp", "stack", "tool"}
    actual:         ["flow", "feat", "stack", "tool", "lang", "mcp", "doc", "app"]
```

**Базовая линия (прогон ДО любых моих правок, на `b1291b06`-состоянии дерева):**
тот же тест уже падал двумя копиями — `PROP-000 package identity list` и
`VIBEVM-SPEC §4.1 installable-kind headings`. Фаза 1 поправила реестр и
PROP-000, но не расширила `formats/vocabularies.json`. Иначе говоря, A2.2 —
атом, который этот часовой чинит, а не ломает.

### A2.2 (содержимое в `005ee978`)

```
$ cargo fmt --all --check
FMT_EXIT=0

$ cargo build --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 52.25s
BUILD_EXIT=0

$ cargo test -p vibe-wire      (15 бинарей; все ok, кроме vocabulary_parity ниже)
test result: ok. 144 passed; 0 failed; ...
test result: ok. 10 passed; ...  (open_vocabulary и остальные)

$ cargo clippy -p vibe-core -p vibe-index -p vibe-cli -p vibe-wire -p vibe-mcp -p xtask --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 33.93s
CLIPPY_EXIT=0

$ cargo xtask check-codegen            (после коммита — он git-дифает generated/)
xtask check-codegen: clean.
CC_EXIT=0
```

Остаток дрейфа после A2.2 — **ровно четыре прозаические копии, все в файлах,
которые пакет запрещает мне трогать либо которые принадлежат поправке реестра
владельца**:

```
$ cargo test -p vibe-wire --test vocabulary_parity
package_kind vocabulary drift from formats/vocabularies.json; every lagging copy is named:

boot core terminology list [vibevm/vibespecs/boot/00-core.xml]
    missing exact enumeration: only six installable kinds — `flow`, `feat`, `stack`, `tool`, `mcp`, `lang`, `doc`, `app`

VIBEVM-SPEC package-identity inline list [VIBEVM-SPEC.md]
    missing exact enumeration: `kind` (`flow` / `feat` / `stack` / `tool` / `mcp` / `lang` / `doc` / `app`) stays

VIBEVM-SPEC manifest-example inline list [VIBEVM-SPEC.md]
    missing exact enumeration: one of: flow, feat, stack, tool, mcp, lang, doc, app — metadata, not identity

VIBEVM-SPEC glossary Kind list [VIBEVM-SPEC.md]
    missing exact enumeration: **Kind.** One of `flow`, `feat`, `stack`, `tool`, `mcp`, `lang`, `doc`, `app`. The category of a package.
```

Каждая строка — это готовый текст замены; тест печатает буквально то, что
должно стоять в файле. Прецедент в `BACKLOG.md` B070: те же три строки
`VIBEVM-SPEC.md` отставали с доPROP-027 времён и были «выправлены боссом той же
посадкой». Рекомендация: это хвост A1.1 (поправка реестра), не A2.2 — четыре
правки на четыре строки, после них часовой зелёный.

### A2.4 (на коммите `29351a68`)

```
$ cargo fmt --all --check
(единственный Diff — crates\vibe-doc\src\examples\jtd.rs, чужой новый крейт;
 ни один из моих файлов не отформатирован неверно)

$ cargo build --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 25.78s

$ cargo test -p vibe-core
test result: ok. 491 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.01s
test result: ok. 192 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.90s

$ cargo test -p vibe-check
test result: ok. 73 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s

$ cargo test -p vibe-install --test doc_kind_refusal
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

$ cargo clippy -p vibe-core -p vibe-check -p vibe-install --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 02s
CLIPPY_EXIT=0

$ cargo xtask specmap
specmap: wrote specmap.json (7930 spec units, 3440 tagged code items, 2985 edges, 0 suspects, 31 warnings).
specmap: ratchet gate — 0 gated orphan(s), 0 dispositioned (6 crate(s) exempt).
specmap: resolve gate — 0 unresolved host edge(s), 35 non-host edge(s) outside this map's jurisdiction.

$ vibe check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0
vibe check: clean — every check passed against
`C:\Users\olegc\git\v\vibevm-docs\vibevm\vibepacks\org.vibevm.core\vibevm-docs\v0.1.0`
```

Для сравнения, тот же `vibe check` **до** атома:

```
vibe check: 2 findings
  [E] [manifest_validity] vibe.toml — unknown field `documents`, expected one of `project`, `package`, …
  [E] [boot_directory] vibevm/vibespecs/boot — … is missing — every project owns this directory
2 errors, 0 warnings, 0 info
```

## Что не сделано и почему

1. **Отдельного коммита `feat(wire): widen package_kind vocabulary for doc and
   app` нет.** Соседний воркер закоммитил весь индекс, пока я стейджил A2.2;
   содержимое целиком внутри `005ee978`. Переписывать историю я не стал: это
   Rule 4 и решение центральной сессии. Варианты для неё: оставить как есть с
   пометкой в леджере, либо расщепить `005ee978` (17 моих файлов перечислены
   в разделе A2.2 — они не пересекаются с четырьмя файлами страниц и
   `PAGE-INDEX.md` того воркера).
2. **Корпуса `formats/corpora/**` не пополнены образцами `kind: doc` / `app`.**
   13 корпусов из A0.2 — это не просто JSON: их закон
   (`crates/vibe-index/tests/wire_parity_index_cli.rs`, `…_index_http.rs`) —
   «настоящий бинарь, прогнанный по **одному общему** фикстурному индексу,
   печатает ровно эти байты». Добавление пакета сдвигает `package_count`,
   `returned`, содержимое `by-name/`, `primary.jsonl` и sha256 в `repomd.json`
   сразу в дюжине файлов и трёх оракулах — это не «пополнить корпус», это
   переписать фикстуру. Свойство, ради которого нужен образец (новое значение
   читается **именованным** вариантом и пишется теми же байтами), пинуется
   напрямую расширенным
   `crates/vibe-wire/tests/open_vocabulary.rs::every_known_value_reads_named_and_writes_identical_bytes`.
   Рекомендация: образцы doc/app в корпусах завести в **A2.3**, где фикстурный
   индекс и так получает doc-пакет с полями связи и карточки.
3. **Четыре прозаические копии словаря** (3 строки `VIBEVM-SPEC.md` + 1 в
   `vibevm/vibespecs/boot/00-core.xml`). `VIBEVM-SPEC.md` пакет запрещает
   трогать; `00-core.xml` не запрещён, но это нормативный boot-файл того же
   жанра, и его фраза «`app` is anticipated» устарела ровно этой поправкой —
   правка того же реестра, что и остальные три. Точный текст замены — в выводе
   часового выше.
4. **`specmap.json` перегенерирован, но НЕ закоммичен.** 0 suspects, гейт
   пройден; в файл попали рёбра сразу нескольких параллельных воркеров, и
   забрать их в свой атомарный коммит было бы неверно. Файл лежит изменённым в
   дереве — его коммитит центральная сессия, когда волна осядет.
5. **`cargo xtask wire-diff` до вердикта не дошёл** — падает на шаге 1 по
   причине, не связанной ни с одним атомом (аномалия №1 ниже). Вердикт выведен
   из таблицы `wire_diff.rs` и флагов `formats/EPOCHS.toml` вручную, щуп шага 2
   прогнан отдельно (вывод выше).
6. **`formats/breaks/005.md` не создан** — не требуется: `public = false`.

## Аномалии продукта, найденные по пути (правок не делал)

1. **`cargo xtask wire-diff` и `cargo test` по всему workspace не могут быть
   зелёными одновременно: golden-корпус индекса зависит от того, кто ещё есть в
   графе сборки.** `xtask` тянет `zip` с фичей `deflate-flate2-zlib-rs`
   (`Cargo.toml:220`), она включает `flate2/zlib-rs`; унификация фич cargo
   переводит **`vibe-index`** на тот же бэкенд, и `primary.jsonl.gz` сжимается
   иначе. Доказательство:
   - `cargo test -p vibe-index --test golden_corpus` → **ok**;
   - `cargo test -p vibe-index -p xtask --test golden_corpus` → **FAILED**,
     `primary.jsonl.gz differs (binary): committed 1364 byte(s), projected 1368 byte(s)`,
     `repomd.json` строка 75 `"size": "1364"` против `"1368"`;
   - `cargo tree -p xtask -e features -i flate2` → `flate2 feature "zlib-rs"`
     ← `zip feature "deflate-flate2-zlib-rs"` ← `xtask`.
   Некомпрессированный `primary.jsonl` при этом **совпадает**, то есть строки
   проекции не менялись; расходится только огибающая gzip. Следствие:
   `cargo xtask wire-diff` на этой машине всегда упирается в шаг 1 и до вердикта
   не доходит. Направления починки: приколотить бэкенд `flate2` для `vibe-index`
   явной фичей; либо снять `zlib-rs` у `zip` в xtask; либо сделать запись `.gz`
   детерминированной независимо от бэкенда. Проверено, что к видам это
   отношения не имеет: в журнале корпуса намеренно неизвестный вид — `"plugin"`,
   а `doc`/`app` там не встречаются.
2. **Третья, несторожимая копия словаря видов.** `schemas/extensions_report.jtd.json`
   держит СВОИ локальные закрытые словари `manifest_kind` и `package_kind`
   (`x-vocabulary: "closed"`), которые дублируют набор видов, но в списке
   `linked_schema` теста `vocabulary_parity` их нет. Если бы я не наткнулся на
   них через компилятор, они молча отстали бы. Стоит либо добавить их в
   часового, либо переучить схему ссылаться на общий словарь.
3. **`cargo xtask check-codegen` красный, пока сгенерированный код не
   закоммичен.** Он делает `run_codegen()` и затем `git diff --exit-code` по
   generated-деревьям (`xtask/src/codegen/mod.rs:421-445`), то есть сравнивает с
   **HEAD**, а не с рабочим деревом. Это не дефект, но его сообщение («the
   schemas moved and the tree did not») уводит: нормальный незакоммиченный
   кодген читается как дрейф. Стоит добавить в текст третью причину.
4. **`crates/vibe-cli/tests/cli_search.rs` — два красных теста на этой машине**:
   `search_without_full_scan_keeps_unconfigured_status` (ждёт 1 unconfigured,
   получает 0) и `search_full_scan_finds_matching_packages_in_github_org`
   (`registries_full_scanned` отсутствует в JSON). Оба про конфигурацию
   реестров, манифесты в них тривиальные и с новыми полями не пересекаются;
   последние коммиты, трогавшие `search`/`vibe-registry`, — досессионные
   (`7b465809` и старше). Похоже на утечку реальной пользовательской
   конфигурации в `UserScratch` на этой машине.
5. **`crates/vibe-install/tests/native_slot_lifecycle.rs` краснеет после
   `cargo build --workspace`**: `prebuilt_slot_native_runs_and_missing_source_record_refuses_without_cargo`
   требует ровно один `vibe_native_loader_fixture.dll`, а полная сборка кладёт
   его и в `target/debug/`, и в `target/debug/deps/`. Тест зависит от того, как
   именно собирали каталог, а не от продукта.
6. **`crates/vibe-index/src/scanner/manifest.rs`** говорил в доккомментарии
   «closed four-variant PackageKind» при шести вариантах — поправлено на
   «eight» заодно (A0.1 отмечал такой же устаревший счёт в `kind.rs`).

## `git status --short` в конце

```
 M Cargo.lock
 M crates/vibe-cli/Cargo.toml
 M crates/vibe-cli/src/cli.rs
 M crates/vibe-cli/src/commands/mod.rs
 M crates/vibe-cli/src/main.rs
 M crates/vibe-cli/tests/cli_facts.rs
 M crates/vibe-cli/tests/cli_mcp_path_parity.rs
 M crates/vibe-cli/tests/cli_redirect.rs
 M crates/vibe-doc/src/lib.rs
 M crates/vibe-install/tests/incremental_in_place.rs
 M crates/vibe-mcp/tests/tools_oracle.rs
 M crates/vibe-mcp/tests/tools_oracle/dispatch_compat.rs
 M crates/vibe-requirements/src/tests_followup.rs
 M crates/vibe-requirements/src/tests_provider.rs
 M crates/vibe-requirements/src/tests_query.rs
 M crates/vibe-workspace/src/bins/tests.rs
 M crates/vibe-workspace/src/freshness.rs
 M specmap.json
 M vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/architecture/traceability.xml
 M vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/authoring/ship-tools-and-mcp-servers.xml
 M vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/authoring/specs-agents-can-cite.xml
 M vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/howto/update-packages.xml
 M vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/howto/use-a-private-registry.xml
 M vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/lifecycle/extensions-and-providers.xml
 M vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/lifecycle/scrape.xml
 M vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/model/versions.xml
 M vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/reference/machine-formats.xml
?? campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O1.md
?? crates/vibe-cli/src/cli/doc.rs
?? crates/vibe-cli/src/commands/doc.rs
?? crates/vibe-doc/src/examples.rs
?? crates/vibe-doc/src/examples/
?? vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/examples/
```

Из этого моё — **только `specmap.json`** (перегенерирован, см. «Что не
сделано» п. 4) и сам этот отчёт. Всё остальное принадлежит параллельным
воркерам (`vibe-doc`, `vibe doc` в CLI, страницы примеров, правки страниц
руководства): часть была в дереве до меня, часть появилась, пока я работал.
Ничего из этого я не трогал и не стейджил.
