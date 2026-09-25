# Отчёт LP-O1 — учебный путь: wire, разбор, проверки — 2026-09-25

Пакет: `campaigns/docs-2026-09/findings/PACKET-LP-O1.md`. Дерево
`C:\Users\olegc\git\v\vibevm`, ветка `main`, HEAD `b7c28f078`. Git — только
читающий: ничего не добавлено в индекс, не закоммичено, не спрятано.

## 1. Файлы

### Изменены

| Файл | Что |
|---|---|
| `schemas/doc_manifest.jtd.json` | `navigation.chapters` (необязательное), определение `navigation_chapter` |
| `crates/vibe-wire/src/generated/doc_manifest/mod.rs` | **только** `cargo xtask codegen` |
| `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/site/src/generated/doc-manifest.ts` | **только** `cargo xtask codegen` |
| `crates/vibe-core/src/manifest/package/documentation.rs` | `NavigationChapterDecl`, поле `NavigationDecl.chapters`, `chapter_id_form_is_valid` |
| `crates/vibe-core/src/manifest/package.rs` | реэкспорт нового типа и хелпера |
| `crates/vibe-core/src/manifest/mod.rs` | реэкспорт `NavigationChapterDecl` |
| `crates/vibe-core/src/manifest/document/validation.rs` | вызов вынесенной грамматики `[navigation]` |
| `crates/vibe-core/src/manifest/document/tests_documentation.rs` | 5 новых тестов |
| `crates/vibe-check/src/checks/doc_package_contract.rs` | `check_learning_path` — покрытие по дереву |
| `crates/vibe-check/src/checks/doc_package_contract/tests.rs` | 2 новых теста + хелпер `write_pages` |
| `crates/vibe-doc/src/pages.rs` | `documents()`, `document_of()`, общий `walk_package()` |
| `crates/vibe-doc/src/html/inline.rs` | `hrefs()` — извлечение адресов той же грамматикой, что рендер |
| `crates/vibe-doc/src/html/links.rs` | `target_document()` — адрес ссылки → документ этого пакета |
| `crates/vibe-doc/src/manifest.rs` | реэкспорт `chapters`, переписан комментарий о двух порядках |
| `crates/vibe-doc/src/manifest/tests.rs` | тест «главы переносятся, `pages` не двигаются» |
| `crates/vibe-doc/src/translations.rs` | `Problem::UnknownChapter`, `unknown_chapters()` |
| `crates/vibe-doc/src/translations/tests.rs` | тест неизвестного `id` главы |
| `crates/vibe-doc/src/lib.rs` | `pub mod chapters;` |
| `crates/vibe-cli/src/cli/doc.rs` | флаги `--chapters`, `--json` |
| `crates/vibe-cli/src/commands/doc.rs` | блок `--chapters` в `run_check`, вынос хелперов окружения |
| `crates/vibe-cli/src/commands/doc/env.rs` | принял хелперы окружения |
| `crates/vibe-cli/src/commands/doc/tests.rs` | тест «замер печатает, но не меняет код выхода» |
| `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/site/src/lib/manifest.ts` | разбор `chapters` |
| `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/site/src/lib/manifest.test.ts` | 2 новых теста |

### Созданы

| Файл | Что |
|---|---|
| `crates/vibe-doc/src/chapters.rs` | замер ссылок вперёд: `Report`, `ForwardLink`, `check`, `measure`, `to_json` |
| `crates/vibe-doc/src/chapters/tests.rs` | 6 тестов замера |
| `crates/vibe-core/src/manifest/document/validation/navigation.rs` | вся грамматика `[navigation]`, вынесена по бюджету длины файла |
| `crates/vibe-doc/src/manifest/navigation.rs` | чтение `[navigation]` в манифест, вынесено по тому же бюджету |

Файлы вне периметра не тронуты. Пакеты руководства
`vibevm/vibepacks/org.vibevm.core/vibevm-docs*/**` в рабочем дереве изменены
центральной сессией (её параллельная работа), не мной; `specmap.json`,
`facts.toml`, `vibevm/vibespecs/**`, компоненты и маршруты сайта и
`site/src/fixtures/**` не изменялись — тест TS правит **копию** фикстуры в
памяти (`JSON.parse(JSON.stringify(fixture()))`), как это делают соседние тесты.

## 2. Семантика как реализована

### 2.1. Wire (`schemas/doc_manifest.jtd.json`)

`navigation.optionalProperties.chapters` — массив `navigation_chapter`,
`"x-default": null` + `"x-empty": "preserve"`. Это единственная пара
аннотаций, которая сохраняет *разницу между отсутствием и пустотой*
(прецедент: `requirements_report.jtd.json` → `Option<Vec<…>>`); отсюда
`pub chapters: Option<Vec<NavigationChapter>>` и `chapters?: NavigationChapter[]`
— поле выдаётся ровно тогда, когда пакет его объявил.

`navigation_chapter`: `x-wire-order ["id","title","pages","appendix"]`;
`properties` = `id`, `title`, `pages` (`"x-empty": "emit"`);
`optionalProperties` = `appendix` (`"x-default": false`) → в Rust
`#[serde(default, skip_serializing_if = "std::ops::Not::not")] pub appendix: bool`,
то есть на проводе **только когда `true`**. `x-wire-order` у `navigation`
расширен до `["pinned","sections","chapters"]` (пас требует полной
перестановки всех членов). Описания цитируют `##NAV-CHAPTERS`,
`##NAV-CHAPTERS-TRANSLATION`, `##NAV-CHAPTERS-CHECKED` и закон слоёв.

Оба сгенерированных файла переписаны только `cargo xtask codegen`, руками не
правились.

### 2.2. Модель и грамматика (`vibe-core`)

```rust
pub struct NavigationChapterDecl {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub appendix: bool,
}
```

`pages: Option<Vec<String>>`, а не `Vec` с `#[serde(default)]` — **решение**:
норма делает разницу между «ключа нет» и «список пуст» нагруженной.
Отсутствие `pages` — законная строка перевода; `pages = []` в переводе — это
всё равно перевод, объявляющий страницы, и он отказывается. С `#[serde(default)]`
эти два состояния неразличимы.

Отказы (дословно, `spec://…` в каждом):

1. пустой `id`:
   «`[[navigation.chapter]]` carries an empty `id` — the id is what a translation
   names the chapter by, and a row without one cannot be renamed or reported on
   (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-CHECKED;
   fix: give the chapter an id of its own, e.g. `id = "start"`)»
2. пустой `title`:
   «`[[navigation.chapter]]` `<id>` carries an empty `title` — the contents shows
   the title over the chapter's pages, and an empty one would number a blank line
   (violates …#NAV-CHAPTERS-CHECKED; fix: give the chapter its name in this
   package's own language)»
3. `id` дважды:
   «`[[navigation.chapter]]` uses the id `<id>` twice — a chapter id is the one
   handle a translation and a report have on a chapter, and two rows under it make
   every mention of it ambiguous (violates …#NAV-CHAPTERS-CHECKED; fix: give each
   chapter an id of its own)»
4. страница не документный путь:
   «`[[navigation.chapter]]` `<id>` names `<page>`, which is not a document path —
   a chapter holds pages spelled exactly as a pin spells them, under the spec root
   with forward slashes and WITHOUT the extension, because one document is served
   as three projections (violates …#NAV-CHAPTERS-CHECKED; fix: write the path
   alone, e.g. `start/what-vibevm-is`)»
5. одна страница дважды в объявлении:
   «the learning path names `<page>` twice — followed from the first page of the
   first chapter to the last, the path meets every page of the package ONCE, and a
   page in two chapters gives a reader two places to be told one thing (violates
   …#NAV-CHAPTERS-CHECKED; fix: leave the page in the chapter it belongs to and
   remove the other mention)»
6. в переводе строка главы с `pages`:
   «`[[navigation.chapter]]` `<id>` lists `pages`, and this package declares
   [translates] — a translation takes the learning path of the documentation it
   adapts and only names its chapters in its own language; a second copy of the
   path would be a second copy of one fact, and the day they disagreed the two
   languages would be two different manuals (violates
   …#NAV-CHAPTERS-TRANSLATION; fix: keep `id` and `title` and drop `pages`; a
   chapter this package does not name keeps the source's title)»

`pages` не массив строк и `appendix` не булево отказываются самим TOML
(`Manifest::parse_str` → `Error::parse_toml`), и это **намеренно**: грамматика
значения TOML — дело TOML, второй её экземпляр в `vibe-core` был бы вторым
ответом на «это список строк?». Тест
`a_chapter_whose_fields_are_the_wrong_shape_does_not_parse` закрепляет, что
отказ есть и называет поле.

`chapter_id_form_is_valid` слабее `section_id_form_is_valid` (только «не
пусто»), и это отмечено в коде: id раздела — это папка дерева страниц и обязан
её писать; глава — единица учебного плана, она законно собирает страницы из
четырёх папок.

Проверка порядка в `validate()`: `[navigation]` читает `self.translates.is_some()`
напрямую, поэтому не зависит от того, что блок `[translates]` стоит выше.

### 2.3. Покрытие по дереву (`vibe check`)

`doc_package_contract::check_learning_path`. Не запускается вовсе, если
`navigation.chapters` пуст **или** пакет объявил `[translates]` (перевод берёт
путь источника и страниц не называет — мерить его покрытие значило бы мерить
покрытие источника).

Две ошибки, зеркальные друг другу, обе со ссылкой на `#NAV-CHAPTERS-CHECKED`:

- «the learning path's chapter `<id>` names `<page>`, and this package carries no
  page at `vibevm/vibespecs/<page>.xml` — a path that walks through a page nobody
  wrote leads a reader to a dead link and numbers a chapter one page longer than
  it is (violates …; fix: correct the path, or drop it from the chapter if the
  page is gone)»
- «this package carries `vibevm/vibespecs/<document>.xml` and no chapter of the
  learning path holds it — followed from the first page of the first chapter to
  the last, the path meets every page of the package, so a page added later must
  be given its place on it rather than left out of the contents in silence
  (violates …; fix: add `<document>` to the chapter it belongs to, in the order a
  reader should meet it)»

Список страниц пакета берётся новой `vibe_doc::pages::documents()` — обход
`vibevm/vibespecs/**/*.xml` **без разбора страниц**. Так страница, которую
пивот не принял, всё равно считается страницей дерева и всё равно должна
получить место на пути; и проверка не платит за разбор 49 файлов ради вопроса о
каталоге. `documents()` и `read_package()` делят один обход (`walk_package`).

### 2.4. Перевод (`vibe doc check --translations`)

Новый вариант `Problem::UnknownChapter { id }`. `Problem::page()` для него
возвращает `vibe.toml` (константа `derived::manifest::MANIFEST`): дефект в
таблице `[navigation]`, и назвать страницу значило бы отправить читателя отчёта
не в тот файл. Строка отчёта:

```
  UNKNOWN CHAPTER <id>
    the translation renames a chapter of the learning path that `<adapts>` does
    not declare; a translation names the source's chapters and adds none of its own
    (spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-TRANSLATION)
```

Главы сравниваются из двух **манифестов**, не из наборов страниц: `check()`
читает `manifest::chapters(&instance.root)` и `manifest::chapters(package_dir)`
и подмешивает результат чистой `unknown_chapters()` в `report.problems`. Чистая
функция тестируется без реестра, склада и чекаута — та же идиома, что у
`compare()`. Источник без пути делает неизвестными все строки перевода, и это
верное чтение, а не особый случай.

### 2.5. Манифест страниц (`vibe-doc`)

Чтение `[navigation]` вынесено в `crates/vibe-doc/src/manifest/navigation.rs`
(`navigation::read`), публичный адрес `vibe_doc::manifest::chapters` сохранён
реэкспортом. Главы переносятся как есть: `pages` строкой, `appendix` как дано;
`Option` снаружи — `None`, когда таблицы `[navigation]` нет вовсе. `layer::order`
и порядок `pages` не тронуты.

Комментарий у сборки манифеста переписан: теперь он говорит, что **два порядка
— это решение** (`##NAV-CHAPTERS-DECISION`), а не то, что нужно разрешить; что
`pages` остаётся законом для `llms`-уровней и MCP; и что проекция, свернувшая
путь обратно в `pages`, отняла бы выбор у обеих аудиторий и подсунула бы кэшу
агентов порядок учебника.

### 2.6. Замер ссылок вперёд (`vibe doc check`)

**Имя селектора — `--chapters`.** Конвенция соседей однозначна: каждый селектор
`vibe doc check` назван существительным того, что он проверяет, и совпадает с
именем модуля библиотеки (`--citations` → `citations::check`, `--translations` →
`translations::check`, `--media` → `media::check`). Поэтому модуль
`vibe_doc::chapters` и флаг `--chapters`. `--path` занят каталогом пакета, так
что назвать селектор «путём» было нельзя.

**Место в выводе** — сразу после `--translations`: оба читают таблицу
`[navigation]` через границу пакета, и оба дёшевы (ни продукта, ни песочницы).
`--chapters` добавлен в список в отказе «needs a check to run» на своё место.

**Отчёт не меняет код выхода.** Это единственный блок `run_check` без
`bail!`, и в коде это сказано словами нормы: страница-ориентир смотрит вперёд
намеренно, гейт здесь научил бы автора не писать такую страницу.

Человеческая форма:

```
  AHEAD start/index -> model/two-trees
chapters: 10 chapter(s), 49 page(s) on the path, 18 link(s) pointing ahead of the reader, 0 unreadable page(s) — a measurement and not a gate
```

Пакет без пути:

```
chapters: this package declares no learning path — its contents is the pages in the order the manifest gives them, and there is nothing to measure
```

**Поле JSON.** Флаг `--json` (`vibe doc check --chapters --json`), документирован
как принадлежащий именно этому селектору — единственному здесь, чей вывод
измерение, а не вердикт, который уже несёт код выхода. Форма:

```json
{
  "chapters": 10,
  "pages": 49,
  "forward_links": [ { "page": "start/index", "target": "model/two-trees" } ],
  "forward_link_count": 18,
  "unreadable": [],
  "measured": true
}
```

`forward_links` — те самые пары «страница → цель», `forward_link_count` —
счётчик рядом, чтобы читающая машина не считала массив. `measured: false` у
пакета без пути: «ни одной ссылки вперёд» и «порядок никто не объявлял» печатают
одну цифру и значат обратное.

**Разбор ссылок — существующей моделью, без нового регэкспа.** Модель конвейера
состоит из двух половин, и обе переиспользованы:

- `crates/vibe-doc/src/html/inline.rs` — грамматика инлайна. Добавлена
  `pub fn hrefs(text) -> Vec<String>`; и `render_linked`, и `hrefs` — это один
  проход `scan()` с одним из двух результатов отброшенным. Поэтому «что видит
  читатель» и «на что смотрит проза» — два ответа ОДНОЙ грамматики: ссылка
  внутри code span не считается ровно потому, что там она и не разметка.
- `crates/vibe-doc/src/html/links.rs` — алгебра адресов. Добавлена
  `pub fn target_document(page, target) -> Option<String>`: путь от файла
  страницы, `.xml` обязателен, выход за пакет отказывается — те же `walk`,
  `split_fragment`, `has_scheme`, которыми `Links::cited` разрешает цитату.
  `None` покрывает четыре вещи, и все четыре — «не страница этого пакета»:
  схема/`spec://`, адрес от корня сайта, один якорь, файл не-страница.

Правило замера: страницы берутся в порядке **пути**; для каждой — ссылки в
порядке прозы; считается ссылка, чья цель стоит на пути позже и **не** лежит в
главе с `appendix = true`. Ссылки на якоря той же страницы и на глоссарий в
приложении не считаются не как особые случаи, а как следствия: якорь не
разрешается ни в какой документ, глоссарий объявлен приложением. Страница вне
пути пропускается (у неё нет позиции, позже которой можно быть); главу,
называющую отсутствующую страницу, замер молчит — это находка `vibe check`, и
повторять её значило бы отчитаться об одном дефекте дважды. Непрочитанные
страницы перечисляются: их ссылки неизвестны, а неизвестность — не отсутствие.

### 2.7. TS-парсер сайта

`navigation()` читает необязательное `chapters` с той же строгостью, что
`sections`: `{ok:false, error}` с путём поля, никаких исключений и ни одной
молча выброшенной главы. Отсутствие `chapters` остаётся отсутствием
(`exactOptionalPropertyTypes`: поле подмешивается спредом), потому что сайт на
этом различии и работает — нет пути значит вид по разделам, как раньше.
`appendix` читается `flag()`: отсутствие — не `false` в чтении, а только в
смысле; не-булево отказывается, а не приводится.

### 2.8. Теги прослеживаемости

Новые публичные элементы помечены так же, как соседние — `specmark::scope!` на
модуль (в `vibe-core` и `vibe-doc` `#[spec(...)]` на типах манифеста не стоит ни
у одного соседа, включая `NavigationSectionDecl`). Новые scope-марки:

- `crates/vibe-doc/src/chapters.rs` → `PROP-057#NAV-CHAPTERS-CHECKED`
- `crates/vibe-core/src/manifest/document/validation/navigation.rs` → `PROP-057#NAV-PINNED`
- `crates/vibe-doc/src/manifest/navigation.rs` → `PROP-057#NAV-PINNED`

`specmap.json` **не** перегенерирован.

## 3. Решения и отклонения

**Отклонений от периметра и запретов нет.** Ниже — решения, которые пакет
оставил мне, и два места, где я сделал больше минимума; оба внутри периметра.

1. **`--chapters` как селектор.** Пакет требовал и «в обычном прогоне печатает
   отчёт», и «имя селектора по конвенции соседних отчётов». Прочитал так:
   «обычный прогон» противопоставлен гейту (отчёт печатается и в зелёном
   прогоне, кода выхода не меняет), а селектор нужен — иначе фраза об его имени
   бессмысленна. Реализовано селектором.
2. **`--json` как флаг этого селектора.** У `vibe doc check` не было ни
   `--json`, ни `--format`; конвенция семьи `vibe doc` — `--format md|json`
   (`doc todo`, `doc diff`), но `--format json` для `--examples`/`--coverage`
   ничего не значит, а добавлять JSON-форму всем отчётам — далеко за периметром.
   Поле пакет просил «отдельное», а не «внутри `doc_todo`»: схема `doc_todo`
   вне периметра. Поэтому узкий `--json`, документированный как принадлежащий
   `--chapters`. Периметр это прямо предусматривает («только если отчёту нужен
   флаг или вывод»).
3. **Отказ «страница главы — не документный путь» (п. 2.2, №4) в списке пакета
   не назван.** Добавил: TOML-набросок пакета говорит «пути документов без
   .xml, **как в pinned**», а `pinned` этот отказ имеет. Без него `start/index.xml`
   прошёл бы `vibe-core` и упал бы в `vibe check` сообщением «страницы нет»,
   что для автора хуже. Единственная идиома на одну операцию.
4. **`pages: Option<Vec<String>>`** вместо `Vec` — см. 2.2. Это единственный
   способ отличить законную строку перевода от `pages = []` в переводе.
5. **Три выноса файлов по бюджету длины (600 строк).** Правило стека Rust
   (`discipline://rust-ai-native-lang/guide#surface-form`) — из прочитанных
   пакетом. Мои правки завели за бюджет два файла и утяжелили третий, уже
   бывший за ним; вынес по настоящим швам, чистым переносом:
   - `validation.rs` 704 → 550 + `validation/navigation.rs` 190 (вся грамматика
     `[navigation]` — один вопрос, который больше нигде в `validate` не задаётся);
   - `commands/doc.rs` 620 → 539, хелперы окружения (`spec_sources`,
     `self_coordinate`, `preferred_language`, `settings_home`, `STORE_DIR`) →
     `commands/doc/env.rs`, модуль, уже названный «что вокруг этого вызова».
     Реэкспорт в `doc.rs` сохранил `super::spec_sources` у четырёх
     братских модулей — ни один вызов не переписан;
   - `vibe-doc/src/manifest.rs` 686 → 586 + `manifest/navigation.rs` 119;
     реэкспорт `pub use navigation::chapters;` сохранил публичный адрес
     `vibe_doc::manifest::chapters`.

   После этого `cargo xtask conform check` не показывает ни одной находки
   `file-length` ни в одном тронутом файле (было 3, включая одну
   предсуществующую). Побочный эффект, который надо назвать: две
   дополнительные scope-марки на `NAV-PINNED` — то есть `specmap --check`
   печатает «edges added: 3», а не 1, как предполагал пакет. Оба новых ребра
   ведут на **уже существующую** единицу `NAV-PINNED` (новых единиц и новых
   неразрешённых адресов нет).
6. **`translations.rs`: убран приватный `document_of`** в пользу
   `pages::document_of` — та же функция в двух местах была бы вторым мнением о
   том, что такое «адрес документа».

**Не сделано (и не входило):** ничего из читалки — колонка-путь,
переключатель, пейджер, страница пакета, пятая настройка `##READER-SETTINGS`,
`##NAV-CHAPTERS-READER`; это пакет LP-O2. Главы в манифестах руководства —
центральная сессия. `specmap.json` и `facts.toml` — не мои.

## 4. Тесты

`vibe-core` (`crates/vibe-core/src/manifest/document/tests_documentation.rs`),
5 тестов: путь парсится целиком с `appendix`; каждый из отказов 1–6 п. 2.2;
неверная форма `pages`/`appendix`; законная строка перевода без `pages` и
отказ при `pages = []` и при `pages = ["…"]`.

`vibe-check` (`…/doc_package_contract/tests.rs`), 2 теста: страница главы вне
пакета и страница пакета вне глав — одним прогоном, по одной находке в каждую
сторону; пакет без глав и перевод не задеты вовсе.

`vibe-doc`: `chapters/tests.rs` — 6 тестов (ссылка вперёд считается, назад нет;
ссылка в `appendix` не считается, и та же ссылка при обычной главе считается —
значит работает марка, а не имя папки; якорь, цитата, внешний адрес и страница
вне пути не считаются; пакет без пути не измерен и говорит это; JSON несёт пары
и счётчик; реальный фикстурный пакет читается и остаётся неизмеренным).
`manifest/tests.rs` — главы переносятся, `appendix` как дано, и порядок `pages`
**сравнивается с порядком того же дерева без пути**, причём путь объявлен в
обратном порядке. `translations/tests.rs` — неизвестный `id` главы перевода:
проблема, её `page()` — `vibe.toml`, строка цитирует
`#NAV-CHAPTERS-TRANSLATION`; частичное переименование и пустой список законны;
источник без пути делает неизвестными все строки.

`vibe-cli` (`commands/doc/tests.rs`): замер печатает и **не** меняет код выхода
— пакет с двухглавным путём и ссылкой вперёд, `run_check` успешен и в
человеческой форме, и в `--json`; `--chapters` назван в отказе «needs a check».

TS (`site/src/lib/manifest.test.ts`), 2 теста: объявленный путь пересекает
границу (пары, `appendix` как дано, отсутствие остаётся отсутствием, пустой
массив остаётся пустым массивом); семь неверных форм строки и неверная форма
самого списка — каждая отказывается путём своего поля.

Доктесты: `NavigationDecl`, `NavigationChapterDecl`, `pages::documents`,
`html::inline::hrefs`, `html::links::target_document`, `manifest::chapters`.

## 5. Самопроверка — вывод дословно

Полный протокол остался во временном каталоге сессии (`verify.txt`) и в
репозиторий не входит; ниже — выдержка.

```
### cargo fmt --all -- --check
EXIT=0
```

```
### cargo test -p vibe-core --lib manifest
test result: ok. 363 passed; 0 failed; 0 ignored; 0 measured; 150 filtered out; finished in 0.70s
EXIT=0
```

Фильтр выбрал мои тесты (не ноль) — по именам:
`a_documentation_may_declare_a_learning_path`,
`a_chapter_needs_an_id_and_a_title_and_may_not_share_either`,
`a_chapter_page_is_a_document_path_named_once_on_the_whole_path`,
`a_chapter_whose_fields_are_the_wrong_shape_does_not_parse`,
`a_translation_names_its_chapters_and_may_not_re_declare_their_pages`.

```
### cargo test -p vibe-check doc_
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 72 filtered out; finished in 0.04s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s
EXIT=0
```

Фильтр `doc_` выбрал мои:
`a_learning_path_must_name_pages_that_exist_and_cover_the_ones_that_do`,
`a_package_that_declares_no_path_and_a_translation_are_both_left_alone`.
Второй `0 passed` — интеграционный бинарь `check_cells_oracle`, у которого под
этим фильтром тестов нет (все 24 моих и соседних — в первом).

```
### cargo test -p vibe-doc
test result: ok. 576 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 70 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
EXIT=0
```

Без фильтра — весь набор, включая 70 доктестов и мои 6 + 1 + 1 юнит-теста.

```
### cargo clippy -p vibe-core -p vibe-check -p vibe-doc --all-targets -- -D warnings
    Checking vibe-doc v1.0.0 (C:\Users\olegc\git\v\vibevm\crates\vibe-doc)
    Checking vibe-check v1.0.0 (C:\Users\olegc\git\v\vibevm\crates\vibe-check)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.50s
EXIT=0
```

```
### cargo build -p vibe-cli
   Compiling vibe-cli v1.0.0 (C:\Users\olegc\git\v\vibevm\crates\vibe-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 27.48s
EXIT=0
```

```
### cargo xtask check-codegen
Error: generated code under C:\Users\olegc\git\v\vibevm\crates/vibe-wire/src/generated / C:\Users\olegc\git\v\vibevm\crates/progress-core/src/generated / C:\Users\olegc\git\v\vibevm\vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.0.0\crates/core-ai-native-specmap/src/generated / C:\Users\olegc\git\v\vibevm\vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/site/src/generated\doc-manifest.ts differs from what this machine's jtd-codegen emits for the current schemas and `formats/REGISTRY.toml`.

Two different things produce that difference, and the recipes are opposites — decide which before committing anything:
1. The schemas moved and the tree did not. Fix: run `cargo xtask codegen` and commit the result.
2. The generator is not the pinned build. This check compares the committed tree against the output of whatever binary was found — the project-local copy under `tools/jtd-codegen/` when present, otherwise `jtd-codegen` on PATH — so a different build reads as drift, and recipe 1 would commit ITS emission over ours. Fix: run `jtd-codegen --version`, compare it with the pin's single home (`vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v1.0.0/README.md`), and install the pinned build per that recipe before regenerating.

The distinction is load-bearing: eight post-processing passes are keyed to the pinned emission shape (`codegen/postproc.rs`), and the diff itself cannot tell you which cause produced it.
EXIT=1
```

**Почему красный и почему это не дефект работы.** `check-codegen` — это
`codegen`, а затем `git diff --exit-code` по сгенерированным каталогам. Git у
меня только читающий (`add`/`commit` запрещены пакетом), поэтому моя свежая,
ещё не проиндексированная регенерация неизбежно читается этим `git diff` как
дрейф. Это ровно «рецепт 1» из самого сообщения: `cargo xtask codegen` уже
выполнен, остаётся **commit** — работа центральной сессии.

Реальное свойство, ради которого гейт существует, проверено отдельно:
регенерация **идемпотентна** пиннутым генератором. Сохранил оба файла,
запустил `cargo xtask codegen` второй раз, сравнил:

```
codegen rerun EXIT=0
IDEMPOTENT: a second codegen changes neither generated file
```

и `git diff --stat` по всем четырём каталогам показывает ровно два файла —
мои:

```
 crates/vibe-wire/src/generated/doc_manifest/mod.rs | 59 +++++++++++++++++--
 .../web/v1.0.0/site/src/generated/doc-manifest.ts  | 67 ++++++++++++++++++++--
 2 files changed, 114 insertions(+), 12 deletions(-)
```

```
### cargo xtask specmap --check
Error: `C:\Users\olegc\git\v\vibevm\specmap.json` is out of date relative to the tree.
  drift: unbumped-hash: `spec://org.vibevm.core/vibevm/common/PROP-019#release-production` content changed while the revision stayed at r1 — editorial, or forgot to bump? (bump `r`, or mark the commit body `spec-editorial: release-production`)
  drift: units added: 10
  drift: edges added: 3
Run `rust-ai-native-specmap` (or your project's wrapper), review the drift, and commit the result.
EXIT=1
```

Это всё, что он печатает — три строки дрейфа; сирот он под `--check` не
называет вовсе (прогон падает на дрейфе раньше, чем доходит до трещотки
сирот и до гейта разрешения). Разбор:

- `unbumped-hash` у `PROP-019#release-production` — предсуществующий, пакет его
  называет.
- `units added: 10` — **ни одной моей**: это 8 единиц, объявленных в PROP-057
  (`NAV-CHAPTERS`, `NAV-CHAPTERS-CHECKED`, `NAV-CHAPTERS-DECISION`,
  `NAV-CHAPTERS-READER`, `NAV-CHAPTERS-TRANSLATION`, `nav-chapters-rejected`,
  `nav-chapters-revisit`, `nav-chapters-why`) и 2 в PROP-059
  (`binary-selection`, `mode-transition`). Проверено прямо: в закоммиченном
  `specmap.json` нет ни одной единицы, упоминающей `nav-chapters`, и ни одной
  из двух PROP-059 — спеки я не трогал.
- `edges added: 3` — мои три scope-марки (п. 2.8). Пакет предполагал одну — к
  `NAV-CHAPTERS*`; две остальные ведут к **существующей** `NAV-PINNED` и
  появились из-за выносов по бюджету длины файла (п. 3.5). Новых единиц,
  неразрешённых адресов и новых сирот не добавлено: все мои публичные элементы
  лежат в модулях со scope-маркой, а обе новые марки указывают на уже
  существующие анкеры PROP-057.

```
### cd vibevm/vibepacks/org.vibevm.doc/web/v1.0.0 && TYPESCRIPT_AI_NATIVE=... node tools/floor.mjs --keep-going
=== prettier --check (floor perimeter: design/src, site/src) ===
=== tsc --noEmit ===
=== tests (node --test) ===
…
ℹ tests 122
ℹ pass 122
ℹ fail 0
=== eslint (floor perimeter: design/src, site/src) ===
=== typescript-ai-native-conform check ===
typescript-ai-native-conform check: 0 finding(s) in scope <workspace> ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report-typescript.sarif.
=== typescript-ai-native-specmap --check ===
typescript-ai-native-specmap --check: clean (0 spec units, 157 tagged code items, 157 edges, 0 suspects, 157 warnings).
typescript-ai-native-specmap: ratchet gate — 0 orphan(s) (0 root(s) exempt).
=== test-gate (xfail-strict) ===
test-gate: 131 results parsed (0 failed, 0 skipped), baseline entries: 0
test-gate: green (xfail-strict).

floor: all green (7 step(s) run, 0 disabled by policy).
EXIT=0
```

### Проверки сверх списка пакета

```
cargo test -p vibe-core --doc          → test result: ok. 203 passed; 0 failed   EXIT=0
cargo test -p vibe-doc --doc           → test result: ok.  70 passed; 0 failed   EXIT=0
cargo test -p vibe-cli --bin vibe commands::doc::
                                       → test result: ok.  34 passed; 0 failed   EXIT=0
cargo xtask conform check              → EXIT=1, 85 находок / 26 новых — все в
                                         файлах, которых я не касался; ни одной
                                         `file-length` в тронутых мной (было 3).
```

`cargo xtask conform check` в списке пакета нет и он красный до меня (стоячий
дрейф кампании: `no-unwrap-in-domain` 48, `unsafe-gate` 12, `file-length` 10 и
прочее в `vibe-cli/commands/vvm`, `vibe-registry`, `vibe-core/manifest/lockfile`
и др.). Запускал ради одного вопроса — не добавил ли я находок; не добавил, и
одну предсуществующую (`vibe-doc/src/manifest.rs`, 686 строк) снял.

## 6. Сквозная проверка на настоящем руководстве

Чтобы не трогать пакет руководства, скопировал его дерево в скратчпад и объявил
там путь из §5 `LEARNING-PATH-DESIGN.md` (10 глав, приложение последним):

```
chapters: 10 chapter(s), 49 page(s) on the path, 18 link(s) pointing ahead of the reader, 0 unreadable page(s) — a measurement and not a gate
```

Все 18 ссылок вперёд — с двух ориентирующих страниц, `start/what-vibevm-is` (5)
и `start/index` (13). Это ровно то, что утверждает дизайн («все — с двух
ориентирующих страниц»), и ровно то, на чём норма отказалась ставить гейт.

Число (18, а не 13 из §6 дизайна) объяснимо и проверено вручную:
`start/index.xml` пишет 13 ссылок вида `../…​.xml`, из них 2 в глоссарий и 1 в
`diagnostics/errors` — все три в главе-приложении, значит не считаются, остаётся
10; плюс 3 ссылки в ту же папку (`install-vibe.xml`, `first-project.xml`,
`what-a-project-contains.xml`) — итого 13 с этой страницы, и 5 с
`start/what-vibevm-is`. Замер §6 дизайна брал скратч-скрипт от 2026-09-25 и, судя
по «20 межстраничных ссылок», считал только `../`-ссылки; пакет же прямо
требует считать «и относительные в той же папке». Плюс корпус с тех пор
сдвинулся: центральная сессия правит страницы руководства прямо сейчас.
Реализовано буквальное правило нормы; расхождение с числом дизайна — в счётном
правиле и в дате, не в поведении.
